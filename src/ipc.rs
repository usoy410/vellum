//! EOF-framed legacy IPC with bounded reads and serialized socket ownership.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

pub fn socket_path() -> PathBuf {
    // Preserve the existing endpoint in this compatibility-focused change.
    PathBuf::from(format!("/run/user/{}/vellum.sock", unsafe { libc::getuid() }))
}

pub struct SocketOwner {
    path: PathBuf,
    identity: (u64, u64),
    // Keep the lock inode on disk: unlinking a flock file allows split ownership.
    _lock: File,
}

fn identity(path: &Path) -> io::Result<(u64, u64)> {
    let m = fs::symlink_metadata(path)?;
    Ok((m.dev(), m.ino()))
}

impl SocketOwner {
    pub fn bind(path: &Path) -> io::Result<(Self, UnixListener)> {
        let lock = OpenOptions::new().read(true).write(true).create(true)
            .mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path.with_extension("sock.lock"))?;
        let m = lock.metadata()?;
        if !m.is_file() || m.uid() != unsafe { libc::getuid() } || m.nlink() != 1 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "unsafe IPC lock file"));
        }
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::new(io::ErrorKind::AddrInUse, "another Vellum instance owns IPC"));
        }
        match fs::symlink_metadata(path) {
            Ok(m) => {
                if !m.file_type().is_socket() || m.uid() != unsafe { libc::getuid() } {
                    return Err(io::Error::new(io::ErrorKind::AlreadyExists, "refusing to replace a non-owned socket path"));
                }
                // Also protect running pre-lock versions of Vellum.
                match UnixStream::connect(path) {
                    Ok(_) => return Err(io::Error::new(io::ErrorKind::AddrInUse, "Vellum socket is active")),
                    Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                        if identity(path)? != (m.dev(), m.ino()) {
                            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "socket changed during startup"));
                        }
                        fs::remove_file(path)?;
                    }
                    Err(e) => return Err(e),
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        let listener = UnixListener::bind(path)?;
        let owner = Self { path: path.into(), identity: identity(path)?, _lock: lock };
        Ok((owner, listener))
    }
}

impl Drop for SocketOwner {
    fn drop(&mut self) {
        if identity(&self.path).ok() == Some(self.identity) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn read_request(stream: &mut UnixStream, timeout: Duration) -> io::Result<String> {
    let deadline = Instant::now() + timeout;
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let remaining = deadline.checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "IPC request deadline exceeded"))?;
        stream.set_read_timeout(Some(remaining))?;
        let count = match stream.read(&mut chunk) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 { break; }
        if bytes.len() + count > MAX_REQUEST_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "IPC request exceeds 64 KiB"));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    if bytes.is_empty() || bytes.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty or NUL-containing IPC request"));
    }
    String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("vellum-ipc-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&p).unwrap(); Self(p)
        }
        fn socket(&self) -> PathBuf { self.0.join("vellum.sock") }
    }
    impl Drop for Dir { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    #[test]
    fn second_instance_preserves_active_socket_and_cleanup_allows_restart() {
        let d=Dir::new(); let p=d.socket();
        let (owner,listener)=SocketOwner::bind(&p).unwrap();
        let before=identity(&p).unwrap();
        assert!(SocketOwner::bind(&p).is_err());
        assert_eq!(identity(&p).unwrap(),before);
        let _client=UnixStream::connect(&p).unwrap(); listener.accept().unwrap();
        drop(listener); drop(owner); assert!(!p.exists());
        SocketOwner::bind(&p).unwrap();
    }
    #[test]
    fn protects_legacy_listener_and_recovers_stale_socket() {
        let d=Dir::new(); let p=d.socket(); let legacy=UnixListener::bind(&p).unwrap();
        assert!(SocketOwner::bind(&p).is_err());
        drop(legacy); SocketOwner::bind(&p).unwrap();
    }
    #[test]
    fn preserves_files_symlinks_and_replacements() {
        let d=Dir::new(); let p=d.socket(); fs::write(&p,b"keep").unwrap();
        assert!(SocketOwner::bind(&p).is_err()); assert_eq!(fs::read(&p).unwrap(),b"keep");
        fs::remove_file(&p).unwrap(); std::os::unix::fs::symlink("missing",&p).unwrap();
        assert!(SocketOwner::bind(&p).is_err()); fs::remove_file(&p).unwrap();
        let (owner,_listener)=SocketOwner::bind(&p).unwrap(); fs::remove_file(&p).unwrap();
        fs::write(&p,b"replacement").unwrap(); drop(owner);
        assert_eq!(fs::read(&p).unwrap(),b"replacement");
    }
    fn request(bytes: Vec<u8>) -> io::Result<String> {
        let (mut reader,mut writer)=UnixStream::pair().unwrap();
        let thread=std::thread::spawn(move || {let _=writer.write_all(&bytes);});
        let result=read_request(&mut reader,Duration::from_secs(1)); drop(reader); thread.join().unwrap(); result
    }
    #[test]
    fn legacy_eof_framing_and_limits() {
        assert_eq!(request(b"file:///tmp/a%20b.html".to_vec()).unwrap(),"file:///tmp/a%20b.html");
        assert!(request(vec![b'a';MAX_REQUEST_BYTES]).is_ok());
        for data in [vec![], vec![0], vec![255], vec![b'a';MAX_REQUEST_BYTES+1]] { assert!(request(data).is_err()); }
    }
    #[test]
    fn simultaneous_startup_has_one_owner() {
        let d=Dir::new(); let barrier=std::sync::Arc::new(std::sync::Barrier::new(4));
        let threads: Vec<_>=(0..4).map(|_| {
            let p=d.socket(); let barrier=barrier.clone();
            std::thread::spawn(move || {
                barrier.wait(); let result=SocketOwner::bind(&p);
                barrier.wait(); result.is_ok()
            })
        }).collect();
        assert_eq!(threads.into_iter().filter_map(|t| t.join().ok()).filter(|ok| *ok).count(),1);
    }
    #[test]
    fn slow_drip_cannot_extend_total_deadline() {
        let (mut reader,mut writer)=UnixStream::pair().unwrap();
        let thread=std::thread::spawn(move || {
            for _ in 0..30 {
                if writer.write_all(b"a").is_err() { break; }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let start=Instant::now();
        assert!(read_request(&mut reader,Duration::from_millis(100)).is_err());
        assert!(start.elapsed()<Duration::from_millis(500));
        drop(reader); thread.join().unwrap();
    }
    #[test]
    fn lock_symlink_is_not_followed() {
        let d=Dir::new(); let p=d.socket(); let target=d.0.join("target");
        fs::write(&target,b"keep").unwrap();
        std::os::unix::fs::symlink(&target,p.with_extension("sock.lock")).unwrap();
        assert!(SocketOwner::bind(&p).is_err());
        assert_eq!(fs::read(target).unwrap(),b"keep");
    }
    #[test]
    fn stalled_client_does_not_block_next_client() {
        let d=Dir::new(); let p=d.socket(); let (_owner,listener)=SocketOwner::bind(&p).unwrap();
        let _stall=UnixStream::connect(&p).unwrap();
        let mut next=UnixStream::connect(&p).unwrap(); next.write_all(b"https://example.org").unwrap(); drop(next);
        let (mut first,_)=listener.accept().unwrap();
        assert!(read_request(&mut first,Duration::from_millis(50)).is_err());
        let (mut second,_)=listener.accept().unwrap();
        assert_eq!(read_request(&mut second,Duration::from_secs(1)).unwrap(),"https://example.org");
    }
}
