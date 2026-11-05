/* Experimental native Wayland wallpaper. No production CLI or WebKit changes. */
#include <gtk/gtk.h>
#include <gtk-layer-shell.h>
#include <glib-unix.h>
#include <math.h>
#include <signal.h>

typedef struct {
    GtkWidget *window, *area;
    GdkMonitor *monitor;
    cairo_surface_t *texture;
    guint id, timer;
    gint64 started;
    double offset, pointer_x, pointer_y;
    gboolean pointer_present, hovered, clicked;
    int width, height, scale;
} Scene;
static GPtrArray *scenes;
static gboolean animate;
static guint next_id;

static double transform(Scene *s, double *x, double *y) {
    double width=gtk_widget_get_allocated_width(s->area);
    double height=gtk_widget_get_allocated_height(s->area);
    double scale=MIN(width/1920., height/1080.);
    *x=(width-1920*scale)/2; *y=(height-1080*scale)/2;
    return scale;
}
static gboolean hit(Scene *s) {
    double x,y,scale=transform(s,&x,&y);
    if(!s->pointer_present || scale<=0) return FALSE;
    double local_x=(s->pointer_x-x)/scale, local_y=(s->pointer_y-y)/scale;
    return local_x>=256+s->offset && local_x<384+s->offset && local_y>=256 && local_y<384;
}
static gboolean draw(GtkWidget *widget, cairo_t *cr, gpointer data) {
    (void)widget; Scene *s=data;
    cairo_set_source_rgb(cr,20./255,36./255,56./255); cairo_paint(cr);
    double x,y,scale=transform(s,&x,&y);
    cairo_translate(cr,x,y); cairo_scale(cr,scale,scale);
    cairo_set_source_surface(cr,s->texture,256+s->offset,256); cairo_paint(cr);
    if(s->hovered || s->clicked) {
        cairo_set_source_rgb(cr,1,s->clicked?.7:1,s->clicked?.2:1);
        cairo_set_line_width(cr,4);
        cairo_rectangle(cr,256+s->offset,256,128,128); cairo_stroke(cr);
    }
    return FALSE;
}
static gboolean pointer(GtkWidget *widget, GdkEvent *event, gpointer data) {
    Scene *s=data;
    s->pointer_present=event->type!=GDK_LEAVE_NOTIFY;
    if(s->pointer_present) gdk_event_get_coords(event,&s->pointer_x,&s->pointer_y);
    gboolean hovered=hit(s), changed=hovered!=s->hovered;
    if(changed) g_print("hover output=%u inside=%d\n",s->id,hovered);
    s->hovered=hovered;
    if(event->type==GDK_BUTTON_PRESS && event->button.button==1 && hovered) {
        s->clicked=!s->clicked; changed=TRUE;
        g_print("click output=%u selected=%d\n",s->id,s->clicked);
    }
    if(changed) gtk_widget_queue_draw(widget);
    return FALSE;
}
static gboolean tick(gpointer data) {
    Scene *s=data;
    s->offset=128*sin((g_get_monotonic_time()-s->started)/1000000.);
    s->hovered=hit(s); gtk_widget_queue_draw(s->area);
    return G_SOURCE_CONTINUE;
}
static void allocated(GtkWidget *widget, GtkAllocation *allocation, gpointer data) {
    Scene *s=data; int scale=gtk_widget_get_scale_factor(widget);
    if(s->width==allocation->width && s->height==allocation->height && s->scale==scale) return;
    s->width=allocation->width; s->height=allocation->height; s->scale=scale;
    s->hovered=hit(s);
    g_print("configured output=%u logical=%dx%d scale=%d\n",s->id,s->width,s->height,scale);
}
static void window_destroyed(GtkWidget *widget, gpointer data) {
    (void)widget; Scene *s=data;
    if(s->timer) {g_source_remove(s->timer); s->timer=0;}
    s->window=NULL;
}
static void free_scene(gpointer data) {
    Scene *s=data;
    if(s->window) gtk_widget_destroy(s->window);
    cairo_surface_destroy(s->texture); g_object_unref(s->monitor);
    g_print("removed output=%u\n",s->id); g_free(s);
}
static void monitor_added(GdkDisplay *display, GdkMonitor *monitor, gpointer data) {
    (void)display; (void)data;
    Scene *s=g_new0(Scene,1); s->monitor=g_object_ref(monitor); s->id=++next_id;
    s->window=gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_decorated(GTK_WINDOW(s->window),FALSE);
    gtk_window_set_accept_focus(GTK_WINDOW(s->window),FALSE);
    gtk_layer_init_for_window(GTK_WINDOW(s->window));
    gtk_layer_set_namespace(GTK_WINDOW(s->window),"vellum-native-experiment");
    gtk_layer_set_monitor(GTK_WINDOW(s->window),monitor);
    gtk_layer_set_layer(GTK_WINDOW(s->window),GTK_LAYER_SHELL_LAYER_BACKGROUND);
    gtk_layer_set_keyboard_mode(GTK_WINDOW(s->window),GTK_LAYER_SHELL_KEYBOARD_MODE_NONE);
    gtk_layer_set_exclusive_zone(GTK_WINDOW(s->window),-1);
    for(int edge=0;edge<GTK_LAYER_SHELL_EDGE_ENTRY_NUMBER;edge++)
        gtk_layer_set_anchor(GTK_WINDOW(s->window),edge,TRUE);
    s->area=gtk_drawing_area_new();
    s->texture=cairo_image_surface_create(CAIRO_FORMAT_ARGB32,128,128);
    cairo_t *cr=cairo_create(s->texture);
    cairo_set_source_rgba(cr,.2,.7,.9,.8); cairo_paint(cr); cairo_destroy(cr);
    g_signal_connect(s->area,"draw",G_CALLBACK(draw),s);
    g_signal_connect(s->area,"size-allocate",G_CALLBACK(allocated),s);
    gtk_widget_add_events(s->area,GDK_POINTER_MOTION_MASK|GDK_BUTTON_PRESS_MASK|GDK_LEAVE_NOTIFY_MASK);
    g_signal_connect(s->area,"motion-notify-event",G_CALLBACK(pointer),s);
    g_signal_connect(s->area,"button-press-event",G_CALLBACK(pointer),s);
    g_signal_connect(s->area,"leave-notify-event",G_CALLBACK(pointer),s);
    g_signal_connect(s->window,"destroy",G_CALLBACK(window_destroyed),s);
    gtk_container_add(GTK_CONTAINER(s->window),s->area);
    s->started=g_get_monotonic_time();
    if(animate) s->timer=g_timeout_add(34,tick,s);
    g_ptr_array_add(scenes,s);
    g_print("added output=%u\n",s->id);
    gtk_widget_show_all(s->window);
}
static void monitor_removed(GdkDisplay *display, GdkMonitor *monitor, gpointer data) {
    (void)display; (void)data;
    for(guint i=0;i<scenes->len;i++) {
        Scene *s=g_ptr_array_index(scenes,i);
        if(s->monitor==monitor) {g_ptr_array_remove_index(scenes,i); return;}
    }
}
static gboolean stop(gpointer data) {
    (void)data; gtk_main_quit(); return G_SOURCE_CONTINUE;
}
int main(int argc,char **argv) {
    if(argc>2 || (argc==2 && !g_str_equal(argv[1],"--animate"))) {
        g_printerr("usage: renderer-layer-shell [--animate]\n"); return 2;
    }
    animate=argc==2;
    gdk_set_allowed_backends("wayland");
    if(!gtk_init_check(&argc,&argv) || !gtk_layer_is_supported()) {
        g_printerr("Requires a Wayland compositor with layer-shell support; no fallback window created.\n");
        return 2;
    }
    scenes=g_ptr_array_new_with_free_func(free_scene);
    GdkDisplay *display=gdk_display_get_default();
    gulong added=g_signal_connect(display,"monitor-added",G_CALLBACK(monitor_added),NULL);
    gulong removed=g_signal_connect(display,"monitor-removed",G_CALLBACK(monitor_removed),NULL);
    for(int i=0;i<gdk_display_get_n_monitors(display);i++)
        monitor_added(display,gdk_display_get_monitor(display,i),NULL);
    guint term=g_unix_signal_add(SIGTERM,stop,NULL), interrupt=g_unix_signal_add(SIGINT,stop,NULL);
    gtk_main();
    g_source_remove(term); g_source_remove(interrupt);
    g_signal_handler_disconnect(display,added); g_signal_handler_disconnect(display,removed);
    g_ptr_array_free(scenes,TRUE);
    gdk_display_flush(display);
    return 0;
}
