ascent::ascent_source! { rules:
    #[provenance(&'static str)] relation source_input(i32);
    #[provenance(&'static str)] relation source_output(i32);
    macro take($x: ident) { source_input($x) }
    source_output(x) <-- take!(x);
}

pub mod nested {
    ascent::ascent_source! { extra:
        #[provenance(&'static str)] relation copy(i32);
        copy(x) <-- source_output(x);
    }
}
