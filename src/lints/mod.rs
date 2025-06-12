pub mod duplicate_arg;
pub mod duplicate_modifier;
pub mod empty_modifier;
pub mod gfx_parameters;
pub mod implicit_parens;
pub mod inaccessible_builtin;
pub mod inaccessible_global;
pub mod incompatible_contexts;
pub mod inconsistent_casing;
pub mod invalid_lhs_assignment;
pub mod loop_as_r_value;
pub mod object_instance_unused;
pub mod overwriting_arg;
pub mod ref_arg_in_incompatible_mod;
pub mod sample_section_without_audio_io;
pub mod shadowed_args;
pub mod slider_out_of_range;
pub mod slider_without_description;
pub mod sliders;
pub mod sprintf_args;
pub mod string_arg_in_local_mod;
pub mod too_many_params;
pub mod unknown_function;
pub mod unnecessary_comma;
pub mod unnecessary_semicolon;
pub mod unreachable_function;
pub mod unused_args;
pub mod unused_function;
pub mod unused_modifier_arg;
pub mod unused_variable;
pub mod useless_expression;
pub mod useless_object;
pub mod useless_ref_arg;
pub mod value_as_namespace;
pub mod wrong_arg_count;
pub mod wrong_context;

use crate::{issue::IssueTracker, program::Program};
type Lint = fn(&Program, &mut IssueTracker);

pub fn get() -> Vec<Lint> {
    vec![
        duplicate_arg::lint,
        duplicate_modifier::lint,
        empty_modifier::lint,
        gfx_parameters::lint,
        implicit_parens::lint,
        inaccessible_builtin::lint,
        inaccessible_global::lint,
        incompatible_contexts::lint,
        inconsistent_casing::lint,
        invalid_lhs_assignment::lint,
        loop_as_r_value::lint,
        overwriting_arg::lint,
        ref_arg_in_incompatible_mod::lint,
        object_instance_unused::lint,
        sample_section_without_audio_io::lint,
        shadowed_args::lint,
        slider_out_of_range::lint,
        sliders::lint,
        slider_without_description::lint,
        sprintf_args::lint,
        string_arg_in_local_mod::lint,
        too_many_params::lint,
        unknown_function::lint,
        unnecessary_comma::lint,
        unnecessary_semicolon::lint,
        unreachable_function::lint,
        unused_args::lint,
        unused_function::lint,
        unused_modifier_arg::lint,
        unused_variable::lint,
        useless_expression::lint,
        useless_object::lint,
        useless_ref_arg::lint,
        value_as_namespace::lint,
        wrong_arg_count::lint,
        wrong_context::lint,
    ]
}
