use ascent_provenance::{provenance, provenance_run, WhyProvenance as W};

provenance! {
    pub struct Included;
    relation before(i32);
    include_source!(source_fixture::rules);
    relation between(i32);
    include_source!(source_fixture::nested::extra);
    relation after(i32);
    before(x), between(x), after(x) <-- copy(x);
}

mod generic {
    use super::*;
    provenance! {
        pub struct Generic<T: Ord + std::hash::Hash>;
        impl<T: Ord + std::hash::Hash + Clone> Generic<T>;
        #[provenance(T)] relation input(i32);
        #[provenance(T)] relation output(i32);
        output(x) <-- input(x);
    }
}

ascent::ascent! {
    struct Ordinary;
    relation input(i32);
    relation output(i32);
    output(x) <-- input(x);
}

#[cfg(feature = "parallel-stock")]
ascent::ascent_par! {
    struct Parallel;
    relation input(i32);
    relation output(i32);
    output(x) <-- input(x);
}

fn main() {
    let tagged = vec![(1, W::token("a"))];
    let mut p = Included::default();
    p.source_input = tagged.clone();
    p.run();
    assert_eq!(p.copy, tagged);
    for rows in [&p.before, &p.between, &p.after] { assert_eq!(rows, &vec![(1,)]); }
    let inline = provenance_run! {
        include_source!(source_fixture::rules);
        include_source!(source_fixture::nested::extra);
        source_input(x) <-- for x in [1];
    };
    assert_eq!(inline.copy[0].1, W::__one());
    let inline = provenance_run! {
        include_source!(source_fixture::rules);
        include_source!(source_fixture::nested::extra);
        #[provenance(&'static str)] relation seed(i32) = tagged.clone();
        source_input(x) <-- seed(x);
    };
    assert_eq!(inline.copy, tagged);
    let mut generic = generic::Generic::<&'static str>::default();
    generic.input = tagged.clone();
    generic.run();
    let owned = std::mem::take(&mut generic.output);
    assert_eq!(owned, tagged);
    let relations = &mut *generic;
    let (input, output) = (&mut relations.input, &mut relations.output);
    input.clear();
    output.extend(owned);
    let mut stock = Ordinary::default();
    stock.input = vec![(1,)];
    stock.run();
    assert_eq!(stock.output, vec![(1,)]);
    #[cfg(feature = "parallel-stock")]
    {
        let mut parallel = Parallel::default();
        parallel.input = vec![(1,)].into_iter().collect();
        parallel.run();
        assert_eq!(parallel.output, vec![(1,)]);
    }
    println!("Independent registry consumer passed");
}
