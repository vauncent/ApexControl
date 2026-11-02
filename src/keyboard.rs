use serde::{Deserialize,Serialize};
use std::sync::atomic::{AtomicU8,Ordering};
#[derive(Clone,Copy,Debug,Default,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Layout {#[default] GermanIso,UsAnsi,SpanishIso,UkIso,FrenchIso,NordicIso,JapaneseJis}
impl Layout {
    pub const ALL:[Self;7]=[Self::GermanIso,Self::UsAnsi,Self::SpanishIso,Self::UkIso,Self::FrenchIso,Self::NordicIso,Self::JapaneseJis];
    pub fn is_iso(self)->bool{!matches!(self,Self::UsAnsi|Self::JapaneseJis)}
    pub fn has_tall_enter(self)->bool{self!=Self::UsAnsi}
    pub fn label(self)->&'static str {crate::i18n::text(match self{Self::GermanIso=>"Deutsch (ISO)",Self::UsAnsi=>"USA (ANSI)",Self::SpanishIso=>"Spanisch (ISO)",Self::UkIso=>"UK (ISO)",Self::FrenchIso=>"Französisch (AZERTY ISO)",Self::NordicIso=>"Nordisch (ISO)",Self::JapaneseJis=>"Japanisch (JIS)"})}
    pub fn from_device(region:Option<u8>,shape:Option<u8>)->Option<Self>{
        match (region,shape){(Some(4),Some(1))=>Some(Self::GermanIso),(Some(3),Some(1))=>Some(Self::UkIso),(_,Some(0))=>Some(Self::UsAnsi),_=>None}
    }
    pub fn legend(self,hid:u8)->Option<&'static str> {
        Some(match (self,hid){
            (Self::FrenchIso,4)=>"Q",(Self::FrenchIso,20)=>"A",(Self::FrenchIso,26)=>"Z",(Self::FrenchIso,29)=>"W",(Self::FrenchIso,16)=>",",(Self::FrenchIso,51)=>"M",
            (Self::FrenchIso,30)=>"&",(Self::FrenchIso,31)=>"é",(Self::FrenchIso,32)=>"\"",(Self::FrenchIso,33)=>"'",(Self::FrenchIso,34)=>"(",(Self::FrenchIso,35)=>"-",(Self::FrenchIso,36)=>"è",(Self::FrenchIso,37)=>"_",(Self::FrenchIso,38)=>"ç",(Self::FrenchIso,39)=>"à",
            (Self::FrenchIso,45)=>")",(Self::FrenchIso,46)=>"=",(Self::FrenchIso,47)=>"^",(Self::FrenchIso,48)=>"$",(Self::FrenchIso,50)=>"*",(Self::FrenchIso,52)=>"ù",(Self::FrenchIso,53)=>"²",(Self::FrenchIso,54)=>";",(Self::FrenchIso,55)=>":",(Self::FrenchIso,56)=>"!",
            (Self::NordicIso,45)=>"+",(Self::NordicIso,46)=>"´",(Self::NordicIso,47)=>"Å",(Self::NordicIso,48)=>"¨",(Self::NordicIso,50)=>"'",(Self::NordicIso,51)=>"Ø",(Self::NordicIso,52)=>"Æ",(Self::NordicIso,53)=>"§",(Self::NordicIso,56)=>"-",
            (Self::JapaneseJis,46)=>"^",(Self::JapaneseJis,47)=>"@",(Self::JapaneseJis,48)=>"[",(Self::JapaneseJis,50)=>"]",(Self::JapaneseJis,51)=>";",(Self::JapaneseJis,52)=>":",
            (Self::JapaneseJis,135)=>"\\",(Self::JapaneseJis,137)=>"¥",(Self::JapaneseJis,136)=>"Kana",(Self::JapaneseJis,138)=>"Henkan",(Self::JapaneseJis,139)=>"Muhenkan",(Self::JapaneseJis,230)=>"Alt",
            (Self::GermanIso,28)=>"Z",(Self::GermanIso,29)=>"Y",(Self::GermanIso,45)=>"ß",(Self::GermanIso,46)=>"´",
            (Self::GermanIso,47)=>"Ü",(Self::GermanIso,48)=>"+",(Self::GermanIso,50)=>"#",(Self::GermanIso,51)=>"Ö",(Self::GermanIso,52)=>"Ä",(Self::GermanIso,53)=>"^",(Self::GermanIso,56)=>"-",(Self::GermanIso,100)=>"<",
            (Self::SpanishIso,45)=>"'",(Self::SpanishIso,46)=>"¡",(Self::SpanishIso,47)=>"`",(Self::SpanishIso,48)=>"+",
            (Self::SpanishIso,50)=>"Ç",(Self::SpanishIso,51)=>"Ñ",(Self::SpanishIso,52)=>"´",(Self::SpanishIso,53)=>"º",(Self::SpanishIso,56)=>"-",(Self::SpanishIso,100)=>"<",
            (Self::UkIso,50)=>"#",(Self::UkIso,100)=>"\\",
            (_,28)=>"Y",(_,29)=>"Z",(_,45)=>"-",(_,46)=>"=",(_,47)=>"[",(_,48)=>"]",(_,49)=>"\\",(_,50)=>"#",(_,51)=>";",(_,52)=>"'",(_,53)=>"`",(_,56)=>"/",(_,100)=>"<",_=>return None,
        })
    }
    pub fn shifted(self,hid:u8)->Option<&'static str>{
        if (30..=39).contains(&hid){
            let symbols=match self{
                Self::GermanIso=>["!","\"","§","$","%","&","/","(",")","="],
                Self::SpanishIso=>["!","\"","·","$","%","&","/","(",")","="],
                Self::NordicIso=>["!","\"","#","¤","%","&","/","(",")","="],
                Self::FrenchIso=>["1","2","3","4","5","6","7","8","9","0"],
                Self::JapaneseJis=>["!","\"","#","$","%","&","'","(",")", ""],
                Self::UkIso=>["!","\"","£","$","%","^","&","*","(",")"],
                Self::UsAnsi=>["!","@","#","$","%","^","&","*","(",")"],
            };return Some(symbols[(hid-30) as usize]).filter(|s|!s.is_empty());
        }
        Some(match (self,hid){
            (Self::GermanIso,45)=>"?",(Self::GermanIso,46)=>"`",(Self::GermanIso,48)=>"*",(Self::GermanIso,50)=>"'",
            (Self::GermanIso|Self::NordicIso,54)=>";",(Self::GermanIso|Self::NordicIso,55)=>":",(Self::GermanIso|Self::NordicIso,56)=>"_",
            (Self::SpanishIso,45)=>"?",(Self::SpanishIso,46)=>"¿",(Self::SpanishIso,47)=>"^",(Self::SpanishIso,48)=>"*",(Self::SpanishIso,54)=>";",(Self::SpanishIso,55)=>":",(Self::SpanishIso,56)=>"_",
            (Self::NordicIso,45)=>"?",(Self::NordicIso,46)=>"`",(Self::NordicIso,48)=>"^",(Self::NordicIso,50)=>"*",
            (Self::FrenchIso,16)=>"?",(Self::FrenchIso,45)=>"°",(Self::FrenchIso,46)=>"+",(Self::FrenchIso,47)=>"¨",(Self::FrenchIso,48)=>"£",(Self::FrenchIso,50)=>"µ",(Self::FrenchIso,52)=>"%",(Self::FrenchIso,54)=>".",(Self::FrenchIso,55)=>"/",(Self::FrenchIso,56)=>"§",
            (Self::JapaneseJis,45)=>"=",(Self::JapaneseJis,46)=>"~",(Self::JapaneseJis,47)=>"`",(Self::JapaneseJis,48)=>"{",(Self::JapaneseJis,50)=>"}",(Self::JapaneseJis,51)=>"+",(Self::JapaneseJis,52)=>"*",(Self::JapaneseJis,135)=>"_",(Self::JapaneseJis,137)=>"|",
            (Self::UsAnsi|Self::UkIso,45)=>"_",(Self::UsAnsi|Self::UkIso,46)=>"+",(Self::UsAnsi|Self::UkIso,47)=>"{",(Self::UsAnsi|Self::UkIso,48)=>"}",(Self::UsAnsi|Self::UkIso,49)=>"|",(Self::UkIso,50)=>"~",(Self::UsAnsi|Self::UkIso,51)=>":",(Self::UsAnsi,52)=>"\"",(Self::UkIso,52)=>"@",
            (Self::UsAnsi|Self::UkIso|Self::JapaneseJis,54)=>"<",(Self::UsAnsi|Self::UkIso|Self::JapaneseJis,55)=>">",(Self::UsAnsi|Self::UkIso|Self::JapaneseJis,56)=>"?",
            (Self::UkIso,100)=>"|",(_,100)=>">",_=>return None,
        })
    }
    pub fn keycap(self,hid:u8)->String{
        match (self,hid){
            (Self::JapaneseJis,139)=>return "Muh.".into(),(Self::JapaneseJis,138)=>return "Hen.".into(),
            (Self::NordicIso,51)=>return "Ø Ö".into(),(Self::NordicIso,52)=>return "Æ Ä".into(),_=>{},
        }
        let main=crate::protocol::key_label(hid);
        if let Some(shift)=self.shifted(hid){format!("{main} {shift}")}else{main}
    }
    pub fn rows(self)->Vec<Vec<(u8,f32)>> {
        let mut rows=vec![
            vec![(41,1.0),(30,1.0),(31,1.0),(32,1.0),(33,1.0),(34,1.0),(35,1.0),(36,1.0),(37,1.0),(38,1.0),(39,1.0),(45,1.0),(46,1.0),(42,2.0)],
            vec![(43,1.5),(20,1.0),(26,1.0),(8,1.0),(21,1.0),(23,1.0),(28,1.0),(24,1.0),(12,1.0),(18,1.0),(19,1.0),(47,1.0),(48,1.0),(40,1.5)],
            vec![(57,1.75),(4,1.0),(22,1.0),(7,1.0),(9,1.0),(10,1.0),(11,1.0),(13,1.0),(14,1.0),(15,1.0),(51,1.0),(52,1.0),(50,1.0),(40,1.25)],
            vec![(225,1.25),(100,1.0),(29,1.0),(27,1.0),(6,1.0),(25,1.0),(5,1.0),(17,1.0),(16,1.0),(54,1.0),(55,1.0),(56,1.0),(229,2.75)],
            vec![(224,1.25),(227,1.25),(226,1.25),(44,6.25),(230,1.25),(231,1.25),(240,1.25),(228,1.25)],
        ];
        if self==Self::UsAnsi{rows[1].last_mut().unwrap().0=49;rows[2].remove(12);rows[2].last_mut().unwrap().1=2.25;rows[3].remove(1);rows[3][0].1=2.25;}
        if self==Self::JapaneseJis{
            rows[0].last_mut().unwrap().1=1.0;rows[0].insert(13,(137,1.0));
            rows[3].remove(1);rows[3][0].1=2.25;rows[3].last_mut().unwrap().1=1.75;rows[3].insert(11,(135,1.0));
            rows[4]=vec![(224,1.25),(227,1.0),(226,1.25),(139,1.0),(44,5.5),(138,1.0),(136,1.0),(230,1.0),(240,1.0),(228,1.0)];
        }rows
    }
}
static LAYOUT:AtomicU8=AtomicU8::new(0);
#[cfg(test)]
thread_local!{static TEST_LAYOUT:std::cell::Cell<Option<Layout>>=const{std::cell::Cell::new(None)};}
pub fn set_layout(layout:Layout){LAYOUT.store(layout as u8,Ordering::Relaxed);}
pub fn current()->Layout{
    #[cfg(test)]
    if let Some(layout)=TEST_LAYOUT.with(|value|value.get()){return layout;}
    Layout::ALL.get(LAYOUT.load(Ordering::Relaxed) as usize).copied().unwrap_or_default()
}
#[cfg(test)]
pub fn with_layout<R>(layout:Layout,body:impl FnOnce()->R)->R{
    struct Reset(Option<Layout>);impl Drop for Reset{fn drop(&mut self){TEST_LAYOUT.with(|value|value.set(self.0));}}
    let _reset=Reset(TEST_LAYOUT.with(|value|value.replace(Some(layout))));body()
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn mini_ansi_and_iso_geometry_preserves_device_key_ids(){
        for layout in Layout::ALL{
            let rows=layout.rows();for row in &rows{assert_eq!(row.iter().map(|(_,width)|width).sum::<f32>(),15.0);}
            let keys:std::collections::BTreeSet<_>=rows.into_iter().flatten().map(|(hid,_)|hid).collect();
            assert_eq!(keys.len(),match layout{Layout::UsAnsi=>61,Layout::JapaneseJis=>65,_=>62});
            assert!(keys.iter().all(|key|crate::protocol::KEY_ORDER.contains(key)));
            assert_eq!(keys.contains(&100),layout.is_iso());assert_eq!(keys.contains(&49),layout==Layout::UsAnsi);
            for key in [135,136,137,138,139]{assert_eq!(keys.contains(&key),layout==Layout::JapaneseJis);}
        }
        assert_eq!(Layout::UsAnsi.rows()[2].last(),Some(&(40,2.25)));
        assert_eq!(Layout::JapaneseJis.rows()[0].last(),Some(&(42,1.0)));
        assert!(Layout::JapaneseJis.rows()[4].contains(&(44,5.5)));
    }
    #[test]
    fn key_legends_follow_keycaps_instead_of_interface_language(){
        crate::i18n::with_language(crate::i18n::Language::English,||{
            with_layout(Layout::GermanIso,||{assert_eq!(crate::protocol::key_label(28),"Z");assert_eq!(crate::protocol::key_label(52),"Ä");});
            with_layout(Layout::UsAnsi,||{assert_eq!(crate::protocol::key_label(28),"Y");assert_eq!(crate::protocol::key_label(52),"'");});
            with_layout(Layout::SpanishIso,||{assert_eq!(crate::protocol::key_label(51),"Ñ");assert_eq!(crate::protocol::key_label(46),"¡");assert_eq!(crate::protocol::key_label(50),"Ç");});
            with_layout(Layout::FrenchIso,||{
                assert_eq!(crate::protocol::key_label(20),"A");assert_eq!(crate::protocol::key_label(4),"Q");assert_eq!(crate::protocol::key_label(51),"M");
                assert_eq!(Layout::FrenchIso.keycap(31),"é 2");
                let letters=crate::protocol::KeyCategory::Letters.keys();assert_eq!(letters.len(),26);assert!(letters.contains(&51));assert!(!letters.contains(&16));
            });
            with_layout(Layout::NordicIso,||{assert_eq!(crate::protocol::key_label(47),"Å");assert_eq!(Layout::NordicIso.keycap(51),"Ø Ö");assert_eq!(Layout::NordicIso.keycap(52),"Æ Ä");});
            with_layout(Layout::JapaneseJis,||{
                assert_eq!(crate::protocol::key_label(137),"¥");assert_eq!(crate::protocol::key_label(136),"Kana");
                assert_eq!(Layout::JapaneseJis.keycap(51),"; +");
                for hid in [135,136,137,138,139]{assert!(crate::protocol::KeyCategory::ALL.iter().any(|category|category.keys().contains(&hid)));}
            });
        });
        assert_eq!(Layout::from_device(Some(4),Some(1)),Some(Layout::GermanIso));
        assert_eq!(Layout::from_device(Some(1),Some(0)),Some(Layout::UsAnsi));
        assert_eq!(Layout::from_device(Some(3),Some(1)),Some(Layout::UkIso));
        assert_eq!(Layout::from_device(Some(255),Some(1)),None);
    }
}
