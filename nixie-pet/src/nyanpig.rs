//! 宠物页由多段静态资源在编译期拼接为完整 HTML，便于维护。
pub const HTML: &str = concat!(
    include_str!("nyanpig-head.html"),
    include_str!("nyanpig.css"),
    include_str!("nyanpig-body.html"),
    include_str!("nyanpig-i18n.js"),
    include_str!("nyanpig.js"),
    include_str!("nyanpig-tail.html"),
);
