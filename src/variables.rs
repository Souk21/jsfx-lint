use std::rc::Rc;
use uuid::Uuid;

use crate::{
    Location, MAX_SLIDER_COUNT, Meta,
    access::{self},
    context::Context,
    rcsubstring::RcSubString,
};

#[derive(Debug)]
pub struct Variable {
    pub accesses: Vec<access::TopLevel>,
    pub name: RcSubString,
    pub uuid: Uuid,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct BuiltinVar {
    pub name: String,
    pub writable: bool,
    pub readable: bool,
    pub context: Option<Context>,
}

/// Represents a identifier that may or may not be a builtin identifier.
pub enum IsBuiltin<'a, 'b> {
    Slider(&'a Meta),
    BuiltIn(&'b Rc<BuiltinVar>),
    /// Identifier that starts with `_global.` (i.e. shared with all JSFXs)
    Global,
    None,
}

/// Represents a identifier that may or may not be a sliderN identifier.
pub enum MaybeSliderNVar<'a> {
    /// `sliderN` that correctly refer to a slider
    Some(&'a Meta),
    /// `sliderN` that seems to refer to a slider, but this slider is also bound to an identifier.
    /// Sliders that are bound to an identifier (aka named), are not bound to "their" `sliderN` identifier,
    /// and using this `sliderN` identifier result in a new global
    Shadowed(&'a Meta),
    /// `sliderN` with `N > MAX_SLIDER_COUNT` or `N == 0`
    LooksLike,
    /// `sliderN` with a correct index (`1 <= N <= MAX_SLIDER_COUNT`) but no slider exist with that N
    NonExisting,
    None,
}

/// Represents a identifier that may or may not be bound to a slider.
pub enum MaybeBoundToSlider<'a> {
    Some(&'a Meta),
    Shadowed,
    None,
}

impl Variable {
    pub fn new(name: &RcSubString) -> Self {
        Self {
            accesses: Vec::new(),
            name: name.clone(),
            uuid: Uuid::new_v4(),
        }
    }
    pub fn first_location(&self) -> &Location {
        self.accesses
            .first()
            .map(|access| &access.info.location)
            .expect("Variable should only exist if accessed at least once")
    }
    pub fn first_read(&self) -> Option<&access::TopLevel> {
        self.accesses.iter().find(|access| access.info.is_read())
    }
    pub fn first_write(&self) -> Option<&access::TopLevel> {
        self.accesses.iter().find(|access| access.info.is_write())
    }
    pub fn is_read(&self) -> bool {
        self.accesses.iter().any(|access| access.info.is_read())
    }
    pub fn is_written(&self) -> bool {
        self.accesses.iter().any(|access| access.info.is_write())
    }
}

pub fn looks_like_slider_n_var<'a>(metas: &'a [Meta], variable_name: &str) -> MaybeSliderNVar<'a> {
    if !variable_name.to_ascii_lowercase().starts_with("slider")
        || variable_name.len() > format!("slider{MAX_SLIDER_COUNT}").len()
    {
        return MaybeSliderNVar::None;
    }
    let slider_id = &variable_name[6..];
    if slider_id.find(|c: char| !c.is_ascii_digit()).is_some() {
        return MaybeSliderNVar::None;
    }
    let slider_id_usize = slider_id.parse::<usize>();
    let Ok(slider_id_usize) = slider_id_usize else {
        return MaybeSliderNVar::None;
    };
    if !(1..=MAX_SLIDER_COUNT).contains(&slider_id_usize) {
        return MaybeSliderNVar::LooksLike;
    }
    let meta = metas.iter().find(
        |m| matches!(m, Meta::SliderPath{id, ..} | Meta::Slider { id, .. } if slider_id == id.as_str()),
    );
    let Some(meta) = meta else {
        return MaybeSliderNVar::NonExisting;
    };
    // sliderN variable are only bound to sliders if they're not bound to an identifier (aka named)
    if let Meta::Slider {
        identifier: None, ..
    }
    | Meta::SliderPath { .. } = meta
    {
        return MaybeSliderNVar::Some(meta);
    }
    MaybeSliderNVar::Shadowed(meta)
}

pub fn is_bound_to_slider<'a>(metas: &'a [Meta], variable_name: &str) -> MaybeBoundToSlider<'a> {
    match looks_like_slider_n_var(metas, variable_name) {
        MaybeSliderNVar::Some(meta) => return MaybeBoundToSlider::Some(meta),
        MaybeSliderNVar::Shadowed(_) => return MaybeBoundToSlider::Shadowed,
        _ => (),
    }

    for meta in metas {
        if matches!(meta, Meta::Slider { identifier: Some(identifier), .. } if variable_name.to_ascii_lowercase() == identifier.to_lower())
        {
            return MaybeBoundToSlider::Some(meta);
        }
    }
    MaybeBoundToSlider::None
}
