//! Input rows retain their source identity, including duplicate logical facts.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub type Id = u32;
pub const FILES: [&str; 4] = ["alloc", "assign", "load", "store"];

#[derive(Clone, Copy, Debug)]
pub enum Row {
   Alloc(Id, Id),
   Assign(Id, Id),
   Load(Id, u32, Id),  // destination, field, base (the TSV order)
   Store(Id, u32, Id), // base, field, source
}
impl Row {
   pub fn vars(self) -> (Id, Id) {
      match self {
         Self::Alloc(a, b) | Self::Assign(a, b) | Self::Load(a, _, b) | Self::Store(a, _, b) => (a, b),
      }
   }
   pub fn kind(self) -> usize {
      match self {
         Self::Alloc(..) => 0,
         Self::Assign(..) => 1,
         Self::Load(..) => 2,
         Self::Store(..) => 3,
      }
   }
}
#[derive(Clone, Debug)]
pub struct Fact {
   pub token: usize,
   pub row: Row,
   pub line: usize,
}
pub struct Facts {
   pub rows: Vec<Fact>,
   pub symbols: Vec<String>,
}

pub fn default_path() -> PathBuf {
   Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../../byods/ascent-byods-rels/examples/steensgaard/openjdk_javalang_steensgaard")
}

impl Facts {
   pub fn parse(inputs: [&str; 4]) -> Result<Self, String> {
      let mut names = HashMap::<String, Id>::new();
      let mut result = Self { rows: vec![], symbols: vec![] };
      for (kind, input) in inputs.into_iter().enumerate() {
         for (line, text) in input.lines().enumerate() {
            let columns: Vec<_> = text.split('\t').collect();
            let error = || {
               format!(
                  "{}:{}: expected {} nonempty TSV columns; field must be u32",
                  FILES[kind],
                  line + 1,
                  if kind < 2 { 2 } else { 3 }
               )
            };
            if columns.len() != if kind < 2 { 2 } else { 3 } || columns.iter().any(|s| s.is_empty()) {
               return Err(error());
            }
            let field = if kind >= 2 { columns[1].parse::<u32>().map_err(|_| error())? } else { 0 };
            let mut intern = |s: &str| {
               *names.entry(s.to_owned()).or_insert_with(|| {
                  let id = result.symbols.len() as Id;
                  result.symbols.push(s.to_owned());
                  id
               })
            };
            let (a, b) = (intern(columns[0]), intern(columns[if kind < 2 { 1 } else { 2 }]));
            let row = match kind {
               0 => Row::Alloc(a, b),
               1 => Row::Assign(a, b),
               2 => Row::Load(a, field, b),
               _ => Row::Store(a, field, b),
            };
            result.rows.push(Fact { token: result.rows.len(), row, line: line + 1 });
         }
      }
      Ok(result)
   }
   pub fn read(path: &Path) -> Result<Self, String> {
      let data: Vec<_> = FILES
         .iter()
         .map(|name| {
            let p = path.join(format!("{name}.facts"));
            std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))
         })
         .collect::<Result<_, _>>()?;
      Self::parse([&data[0], &data[1], &data[2], &data[3]])
   }
   pub fn counts(&self) -> [usize; 4] {
      let mut counts = [0; 4];
      for fact in &self.rows {
         counts[fact.row.kind()] += 1;
      }
      counts
   }
   fn constraints(&self) -> Groups {
      let mut groups = Groups::default();
      for fact in &self.rows {
         let (a, b) = fact.row.vars();
         groups.union(a, b);
      }
      groups
   }
   pub fn bounded(&self, cap: usize) -> Self {
      if cap == 0 {
         return Self { rows: self.rows.clone(), symbols: self.symbols.clone() };
      }
      let groups = self.constraints();
      let mut counts = HashMap::<_, usize>::new();
      for f in &self.rows {
         *counts.entry(groups.root(f.row.vars().0).unwrap()).or_default() += 1;
      }
      Self {
         rows: self.rows.iter().filter(|f| counts[&groups.root(f.row.vars().0).unwrap()] <= cap).cloned().collect(),
         symbols: self.symbols.clone(),
      }
   }
   pub fn component(&self, seed: Id) -> Self {
      let groups = self.constraints();
      Self {
         rows: self.rows.iter().filter(|f| groups.root(f.row.vars().0) == groups.root(seed)).cloned().collect(),
         symbols: self.symbols.clone(),
      }
   }
}

// Union by size. Only inserted nodes have reflexive pairs: parsing a symbol does
// not make it a member of the points-to relation.
#[derive(Default)]
pub struct Groups {
   parent: HashMap<Id, Id>,
   size: HashMap<Id, usize>,
}
impl Groups {
   pub fn root(&self, mut x: Id) -> Option<Id> {
      loop {
         let p = *self.parent.get(&x)?;
         if p == x {
            return Some(x);
         }
         x = p;
      }
   }
   fn same(&self, a: Id, b: Id) -> bool { self.root(a).is_some_and(|r| Some(r) == self.root(b)) }
   fn union(&mut self, a: Id, b: Id) -> bool {
      let mut changed = false;
      for x in [a, b] {
         if let std::collections::hash_map::Entry::Vacant(entry) = self.parent.entry(x) {
            entry.insert(x);
            self.size.insert(x, 1);
            changed = true;
         }
      }
      let (mut a, mut b) = (self.root(a).unwrap(), self.root(b).unwrap());
      if a == b {
         return changed;
      }
      if self.size[&a] < self.size[&b] {
         std::mem::swap(&mut a, &mut b);
      }
      self.parent.insert(b, a);
      *self.size.get_mut(&a).unwrap() += self.size.remove(&b).unwrap();
      true
   }
   pub fn classes(&self) -> BTreeMap<Id, Vec<Id>> {
      let mut result = BTreeMap::<_, Vec<_>>::new();
      for x in self.parent.keys() {
         result.entry(self.root(*x).unwrap()).or_default().push(*x);
      }
      result
   }
   pub fn count(&self) -> usize { self.size.values().map(|n| n * n).sum() }
   pub fn pairs(&self) -> BTreeSet<(Id, Id)> {
      self.classes().values().flat_map(|xs| xs.iter().flat_map(|a| xs.iter().map(move |b| (*a, *b)))).collect()
   }
}

/// Independent imperative fixed point, with no Ascent or provenance operations.
pub fn reference<'a>(rows: impl IntoIterator<Item = &'a Fact>) -> Groups {
   let rows: Vec<_> = rows.into_iter().collect();
   let mut groups = Groups::default();
   for fact in &rows {
      if let Row::Alloc(a, b) | Row::Assign(a, b) = fact.row {
         groups.union(a, b);
      }
   }
   loop {
      let mut changed = false;
      for store in &rows {
         if let Row::Store(x, f, y) = store.row {
            for load in &rows {
               if let Row::Load(p, g, q) = load.row {
                  if f == g && groups.same(x, q) {
                     changed |= groups.union(y, p);
                  }
               }
            }
         }
      }
      if !changed {
         return groups;
      }
   }
}
