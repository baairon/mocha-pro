//! The kinds of parameter an effect can declare.

/// A parameter kind as a plain value. The text names are the file format's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParamKind {
    Float,
    Int,
    Checkbox,
    Popup,
    Button,
    Layer,
    Point,
    Angle,
    Color,
    Group,
    GroupEnd,
    Arbitrary,
    NoData,
}

impl ParamKind {
    pub const ALL: [ParamKind; 13] = [
        ParamKind::Float,
        ParamKind::Int,
        ParamKind::Checkbox,
        ParamKind::Popup,
        ParamKind::Button,
        ParamKind::Layer,
        ParamKind::Point,
        ParamKind::Angle,
        ParamKind::Color,
        ParamKind::Group,
        ParamKind::GroupEnd,
        ParamKind::Arbitrary,
        ParamKind::NoData,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ParamKind::Float => "float",
            ParamKind::Int => "int",
            ParamKind::Checkbox => "checkbox",
            ParamKind::Popup => "popup",
            ParamKind::Button => "button",
            ParamKind::Layer => "layer",
            ParamKind::Point => "point",
            ParamKind::Angle => "angle",
            ParamKind::Color => "color",
            ParamKind::Group => "group",
            ParamKind::GroupEnd => "group_end",
            ParamKind::Arbitrary => "arbitrary",
            ParamKind::NoData => "nodata",
        }
    }

    pub fn parse(s: &str) -> Option<ParamKind> {
        ParamKind::ALL.iter().copied().find(|k| k.as_str() == s)
    }
}
