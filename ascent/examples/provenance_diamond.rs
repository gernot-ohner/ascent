use ascent::{HowProvenance, ascent};

ascent! {
   #![provenance(HowProvenance<String>)]

   struct Diamond;

   relation edge(i32, i32);
   relation path(i32, i32);

   path(x, z) <-- edge(x, y), edge(y, z);
}

fn main() {
   let mut program = Diamond {
      edge: vec![
         (1, 2, HowProvenance::token("x1".to_owned())),
         (1, 4, HowProvenance::token("x2".to_owned())),
         (2, 3, HowProvenance::token("x3".to_owned())),
         (4, 3, HowProvenance::token("x4".to_owned())),
      ],
      ..Default::default()
   };

   program.run();

   let (_, _, provenance) = program.path.iter().find(|row| (row.0, row.1) == (1, 3)).unwrap();
   println!("path(1, 3): {provenance}");
   assert_eq!(provenance.to_string(), "x1*x3 + x2*x4");
}
