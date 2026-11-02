use std::{collections::{BTreeMap,VecDeque},sync::{Arc,Mutex}};
pub type Frame=Vec<(u8,[u8;3])>;
pub type Preview=Arc<Mutex<Option<(String,Frame)>>>;
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub enum Mode{#[default] Static,Breathing,Wave,Ripple}
impl Mode{
    pub const ALL:[Self;4]=[Self::Static,Self::Breathing,Self::Wave,Self::Ripple];
    pub fn label(self)->&'static str{crate::i18n::text(match self{Self::Static=>"Statisch",Self::Breathing=>"Atmen",Self::Wave=>"Regenbogenwelle",Self::Ripple=>"Reaktive Welle"})}
}
#[derive(Clone,Copy,Debug)]
pub struct Position{pub hid:u8,pub x:f32,pub y:f32}
pub fn positions(layout:crate::keyboard::Layout)->Vec<Position>{
    let mut map:BTreeMap<u8,(f32,f32,f32)>=BTreeMap::new();
    for (row,keys) in layout.rows().into_iter().enumerate(){let mut x=0.0;for (hid,width) in keys{
        let point=map.entry(hid).or_default();point.0+=(x+width/2.0)*width;point.1+=(row as f32+0.5)*width;point.2+=width;x+=width;
    }}map.into_iter().map(|(hid,(x,y,weight))|Position{hid,x:x/weight,y:y/weight}).collect()
}
#[derive(Clone)]
pub struct Config{pub mode:Mode,pub color:[u8;3],pub brightness:u8,pub speed:f32,pub keys:Vec<Position>,pub origins:Vec<Position>}
pub struct Effect{pub config:Config,pulses:VecDeque<(Position,f32)>}
impl Effect{
    pub fn new(config:Config)->Result<Self,String>{
        let valid=|points:&[Position]|!points.is_empty() && points.len()<=68 && points.iter().all(|p|crate::protocol::KEY_ORDER.contains(&p.hid)&&p.x.is_finite()&&p.y.is_finite()&&(0.0..=15.0).contains(&p.x)&&(0.0..=5.0).contains(&p.y)) && points.iter().map(|p|p.hid).collect::<std::collections::BTreeSet<_>>().len()==points.len();
        if !valid(&config.keys)||!valid(&config.origins)||!config.speed.is_finite()||!(0.2..=3.0).contains(&config.speed)||config.brightness>100{return Err(crate::i18n::text("Ungültige RGB-Einstellungen").into());}
        Ok(Self{config,pulses:VecDeque::new()})
    }
    pub fn pulse(&mut self,mut hid:u8,time:f32){
        if hid==49 && !self.config.origins.iter().any(|p|p.hid==49){hid=50;}
        if let Some(origin)=self.config.origins.iter().find(|p|p.hid==hid).copied(){
            if self.pulses.len()==32{self.pulses.pop_front();}self.pulses.push_back((origin,time));
        }
    }
    pub fn frame(&mut self,time:f32)->Frame{
        self.pulses.retain(|(_,start)|time-*start<=2.5);
        self.config.keys.iter().map(|key|{
            let mut color=self.config.color;let factor=match self.config.mode{
                Mode::Static=>1.0,
                Mode::Breathing=>0.5+0.5*(time*self.config.speed*std::f32::consts::TAU/2.0).cos(),
                Mode::Wave=>{color=rainbow(key.x/15.0-time*self.config.speed/3.0);1.0},
                Mode::Ripple=>self.pulses.iter().map(|(origin,start)|{
                    let age=(time-start).max(0.0);let distance=((key.x-origin.x).powi(2)+(key.y-origin.y).powi(2)).sqrt();
                    (1.0-(distance-age*8.0*self.config.speed).abs()/1.1).max(0.0)*(1.0-age/2.5).max(0.0)
                }).fold(0.0_f32,f32::max),
            };
            (key.hid,color.map(|c|(c as f32*factor*self.config.brightness as f32/100.0).round().clamp(0.0,255.0) as u8))
        }).collect()
    }
}
fn rainbow(hue:f32)->[u8;3]{
    let h=hue.rem_euclid(1.0)*6.0;let x=1.0-(h%2.0-1.0).abs();
    let rgb=match h as u8{0=>[1.0,x,0.0],1=>[x,1.0,0.0],2=>[0.0,1.0,x],3=>[0.0,x,1.0],4=>[x,0.0,1.0],_=>[1.0,0.0,x]};rgb.map(|c|(c*255.0).round() as u8)
}
pub fn report(frame:&Frame)->Result<Vec<u8>,String>{
    if frame.is_empty()||frame.len()>68||frame.iter().any(|(hid,_)|!crate::protocol::KEY_ORDER.contains(hid))||frame.iter().map(|(hid,_)|hid).collect::<std::collections::BTreeSet<_>>().len()!=frame.len(){return Err(crate::i18n::text("Bitte gültige Tasten für die Beleuchtung auswählen").into());}
    let mut bytes=vec![0x40,frame.len() as u8];for (hid,color) in frame{bytes.push(*hid);bytes.extend(color);}Ok(bytes)
}
#[cfg(test)] mod tests{
    use super::*;
    fn config(mode:Mode)->Config{let origins=positions(crate::keyboard::Layout::GermanIso);Config{mode,color:[10,200,30],brightness:100,speed:1.0,keys:origins.iter().filter(|p|[5,17].contains(&p.hid)).copied().collect(),origins}}
    #[test]fn static_reports_only_selected_keys_with_exact_colors(){let mut e=Effect::new(config(Mode::Static)).unwrap();let f=e.frame(0.0);assert_eq!(f,vec![(5,[10,200,30]),(17,[10,200,30])]);assert_eq!(report(&vec![(5,[0,255,0])]).unwrap(),vec![0x40,1,5,0,255,0]);assert!(report(&vec![(5,[0;3]),(5,[1;3])]).is_err());}
    #[test]fn breathing_wave_and_reactive_frames_change_without_expanding_the_selection(){
        for mode in [Mode::Breathing,Mode::Wave,Mode::Ripple]{let mut effect=Effect::new(config(mode)).unwrap();let initial=effect.frame(0.0);effect.pulse(5,0.0);let later=effect.frame(if mode==Mode::Ripple{0.05}else{0.7});assert_ne!(initial,later);assert_eq!(later.iter().map(|p|p.0).collect::<Vec<_>>(),vec![5,17]);}
        let mut e=Effect::new(config(Mode::Ripple)).unwrap();for _ in 0..1000{e.pulse(5,0.0);}assert_eq!(e.pulses.len(),32);assert!(e.frame(3.0).iter().all(|(_,rgb)|*rgb==[0;3]));assert!(e.pulses.is_empty());
    }
    #[test]fn geometry_is_deduplicated_and_invalid_effects_are_rejected(){for layout in crate::keyboard::Layout::ALL{let points=positions(layout);assert_eq!(points.iter().filter(|p|p.hid==40).count(),1);}let mut c=config(Mode::Wave);c.speed=f32::NAN;assert!(Effect::new(c).is_err());}
}
