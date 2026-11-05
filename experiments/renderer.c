/* Comparative integration spike, not a production wallpaper backend. */
#include <gtk/gtk.h>
#include <math.h>
#ifdef WEB
#include <webkit2/webkit2.h>
#else
static double offset;
static gint64 started;
static gboolean hovered, clicked;
static cairo_surface_t *texture;
static gboolean draw(GtkWidget *w, cairo_t *cr, gpointer data) {
    (void)w; (void)data;
    cairo_set_source_rgb(cr, 20./255, 36./255, 56./255); cairo_paint(cr);
    cairo_set_source_surface(cr, texture, 256+offset, 256); cairo_paint(cr);
    if (hovered || clicked) {
        cairo_set_source_rgb(cr, 1, clicked ? .7 : 1, clicked ? .2 : 1);
        cairo_rectangle(cr, 256+offset, 256, 128, 128);
        cairo_set_line_width(cr, 4); cairo_stroke(cr);
    }
    return FALSE;
}
static gboolean tick(gpointer widget) {
    offset=128*sin((g_get_monotonic_time()-started)/1000000.);
    gtk_widget_queue_draw(widget); return G_SOURCE_CONTINUE;
}
static gboolean pointer(GtkWidget *w, GdkEvent *e, gpointer data) {
    (void)data; double x=0,y=0;
    gboolean inside=gdk_event_get_coords(e,&x,&y) && x>=256+offset && x<384+offset && y>=256 && y<384;
    if (e->type==GDK_LEAVE_NOTIFY) inside=FALSE;
    hovered=inside;
    if(e->type==GDK_BUTTON_PRESS && inside) {clicked=!clicked; g_print("click delivered\n");}
    gtk_widget_queue_draw(w); return FALSE;
}
#endif
int main(int argc,char **argv) {
    gtk_init(&argc,&argv);
    GtkWidget *window=gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_default_size(GTK_WINDOW(window),1920,1080);
    g_signal_connect(window,"destroy",G_CALLBACK(gtk_main_quit),NULL);
#ifdef WEB
    GtkWidget *content=webkit_web_view_new();
    if(argc!=2) {g_printerr("expected file URI, optionally #animate\n");return 2;}
    webkit_web_view_load_uri(WEBKIT_WEB_VIEW(content),argv[1]);
#else
    GtkWidget *content=gtk_drawing_area_new();
    texture=cairo_image_surface_create(CAIRO_FORMAT_ARGB32,128,128);
    cairo_t *cr=cairo_create(texture);
    cairo_set_source_rgba(cr,.2,.7,.9,.8); cairo_paint(cr); cairo_destroy(cr);
    g_signal_connect(content,"draw",G_CALLBACK(draw),NULL);
    gtk_widget_add_events(content,GDK_POINTER_MOTION_MASK|GDK_BUTTON_PRESS_MASK|GDK_LEAVE_NOTIFY_MASK);
    g_signal_connect(content,"motion-notify-event",G_CALLBACK(pointer),NULL);
    g_signal_connect(content,"button-press-event",G_CALLBACK(pointer),NULL);
    g_signal_connect(content,"leave-notify-event",G_CALLBACK(pointer),NULL);
    started=g_get_monotonic_time();
    if(argc==2 && g_str_equal(argv[1],"--animate")) g_timeout_add(34,tick,content);
#endif
    gtk_container_add(GTK_CONTAINER(window),content);
    gtk_widget_show_all(window); gtk_main();
#ifndef WEB
    cairo_surface_destroy(texture);
#endif
    return 0;
}
