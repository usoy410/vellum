/* GTK4/GSK integration spike. Deliberately not a production wallpaper backend. */
#include <gtk/gtk.h>
#include <math.h>

typedef struct {
    GtkWidget parent;
    GdkTexture *texture;
    double offset, pointer_x, pointer_y;
    gint64 started;
    gboolean pointer_present, hovered, clicked;
    guint timer;
} Scene;
typedef struct { GtkWidgetClass parent; } SceneClass;
G_DEFINE_TYPE(Scene, scene, GTK_TYPE_WIDGET)

static gboolean hit(Scene *s) {
    return s->pointer_present && s->pointer_x >= 256+s->offset &&
        s->pointer_x < 384+s->offset && s->pointer_y >= 256 && s->pointer_y < 384;
}
static void scene_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
    Scene *s = (Scene *)widget;
    const GdkRGBA background = {20.f/255, 36.f/255, 56.f/255, 1};
    graphene_rect_t bounds = GRAPHENE_RECT_INIT(0, 0,
        gtk_widget_get_width(widget), gtk_widget_get_height(widget));
    gtk_snapshot_append_color(snapshot, &background, &bounds);
    bounds = (graphene_rect_t)GRAPHENE_RECT_INIT(256+s->offset, 256, 128, 128);
    gtk_snapshot_append_texture(snapshot, s->texture, &bounds);
    if (s->hovered || s->clicked) {
        const GdkRGBA color = {1, s->clicked ? .7f : 1, s->clicked ? .2f : 1, 1};
        /* Four strips match the centered four-pixel Cairo outline. */
        graphene_rect_t edges[] = {
            GRAPHENE_RECT_INIT(254+s->offset,254,132,4),
            GRAPHENE_RECT_INIT(254+s->offset,382,132,4),
            GRAPHENE_RECT_INIT(254+s->offset,258,4,124),
            GRAPHENE_RECT_INIT(382+s->offset,258,4,124)
        };
        for (unsigned i=0; i<G_N_ELEMENTS(edges); i++)
            gtk_snapshot_append_color(snapshot, &color, &edges[i]);
    }
}
static void motion(GtkEventControllerMotion *controller, double x, double y, gpointer data) {
    (void)controller; Scene *s=data;
    s->pointer_present=TRUE; s->pointer_x=x; s->pointer_y=y;
    gboolean hovered=hit(s);
    if (hovered != s->hovered) {
        s->hovered=hovered; gtk_widget_queue_draw(GTK_WIDGET(s));
    }
}
static void leave(GtkEventControllerMotion *controller, gpointer data) {
    (void)controller; Scene *s=data; s->pointer_present=FALSE;
    if (s->hovered) {s->hovered=FALSE; gtk_widget_queue_draw(GTK_WIDGET(s));}
}
static void pressed(GtkGestureClick *gesture, int count, double x, double y, gpointer data) {
    (void)gesture; (void)count; Scene *s=data;
    s->pointer_present=TRUE; s->pointer_x=x; s->pointer_y=y;
    if (hit(s)) {
        s->clicked=!s->clicked; gtk_widget_queue_draw(GTK_WIDGET(s));
        g_print("click delivered\n");
    }
}
static gboolean tick(gpointer data) {
    Scene *s=data;
    s->offset=128*sin((g_get_monotonic_time()-s->started)/1000000.);
    s->hovered=hit(s); gtk_widget_queue_draw(GTK_WIDGET(s));
    return G_SOURCE_CONTINUE;
}
static void scene_dispose(GObject *object) {
    Scene *s=(Scene *)object;
    if(s->timer) {g_source_remove(s->timer); s->timer=0;}
    g_clear_object(&s->texture);
    G_OBJECT_CLASS(scene_parent_class)->dispose(object);
}
static void scene_class_init(SceneClass *klass) {
    GTK_WIDGET_CLASS(klass)->snapshot=scene_snapshot;
    G_OBJECT_CLASS(klass)->dispose=scene_dispose;
}
static void scene_init(Scene *s) {
    guchar *pixels=g_malloc(128*128*4);
    for(unsigned i=0;i<128*128;i++) {
        pixels[4*i]=51; pixels[4*i+1]=179; pixels[4*i+2]=230; pixels[4*i+3]=204;
    }
    GBytes *bytes=g_bytes_new_take(pixels,128*128*4);
    s->texture=GDK_TEXTURE(gdk_memory_texture_new(128,128,GDK_MEMORY_R8G8B8A8,bytes,128*4));
    g_bytes_unref(bytes);
    GtkEventController *pointer=gtk_event_controller_motion_new();
    g_signal_connect(pointer,"enter",G_CALLBACK(motion),s);
    g_signal_connect(pointer,"motion",G_CALLBACK(motion),s);
    g_signal_connect(pointer,"leave",G_CALLBACK(leave),s);
    gtk_widget_add_controller(GTK_WIDGET(s),pointer);
    GtkGesture *click=gtk_gesture_click_new();
    g_signal_connect(click,"pressed",G_CALLBACK(pressed),s);
    gtk_widget_add_controller(GTK_WIDGET(s),GTK_EVENT_CONTROLLER(click));
    s->started=g_get_monotonic_time();
}
static gboolean closed(GtkWindow *window, gpointer loop) {
    (void)window; g_main_loop_quit(loop); return FALSE;
}
int main(int argc, char **argv) {
    if(argc>2 || (argc==2 && !g_str_equal(argv[1],"--animate"))) {
        g_printerr("usage: renderer-gtk4 [--animate]\n"); return 2;
    }
    gtk_init();
    GMainLoop *loop=g_main_loop_new(NULL,FALSE);
    GtkWidget *window=gtk_window_new();
    gtk_window_set_title(GTK_WINDOW(window),"Vellum GTK4 experiment");
    gtk_window_set_default_size(GTK_WINDOW(window),1920,1080);
    Scene *scene=g_object_new(scene_get_type(),NULL);
    gtk_window_set_child(GTK_WINDOW(window),GTK_WIDGET(scene));
    if(argc==2) scene->timer=g_timeout_add(34,tick,scene);
    g_signal_connect(window,"close-request",G_CALLBACK(closed),loop);
    gtk_window_present(GTK_WINDOW(window));
    GskRenderer *renderer=gtk_native_get_renderer(GTK_NATIVE(window));
    g_print("renderer=%s\n",G_OBJECT_TYPE_NAME(renderer));
    g_main_loop_run(loop); g_main_loop_unref(loop);
    return 0;
}
