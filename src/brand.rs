use eframe::egui::{self,Color32,Pos2,Rect,Stroke,Vec2};
pub fn icon_rgba()->Vec<u8>{include_bytes!("../assets/logo-32.rgba").to_vec()}
pub fn paint(painter:&egui::Painter,rect:Rect){
    let scale=rect.width().min(rect.height())/64.0;
    let origin=rect.center()-Vec2::splat(32.0*scale);
    let point=|x:f32,y:f32|Pos2::new(origin.x+x*scale,origin.y+y*scale);
    painter.rect_filled(Rect::from_min_size(origin,Vec2::splat(64.0*scale)),(14.0*scale) as u8,Color32::from_rgb(24,27,34));
    let hex=vec![point(32.0,7.0),point(54.0,20.0),point(54.0,44.0),point(32.0,57.0),point(10.0,44.0),point(10.0,20.0)];
    painter.add(egui::Shape::closed_line(hex,Stroke::new(3.0*scale,Color32::from_rgb(255,143,79))));
    let white=Stroke::new(4.0*scale,Color32::from_rgb(242,244,248));
    painter.add(egui::Shape::line(vec![point(21.0,43.0),point(32.0,20.0),point(43.0,43.0)],white));
    painter.line_segment([point(26.0,34.0),point(38.0,34.0)],white);
}
