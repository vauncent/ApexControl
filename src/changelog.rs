pub fn text()->&'static str{
    match crate::i18n::language(){
        crate::i18n::Language::German=>include_str!("../CHANGELOG.de.md"),
        crate::i18n::Language::English=>include_str!("../CHANGELOG.md"),
    }
}
