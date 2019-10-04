use petgraph::Graph;
use rls_data::{Analysis, Def, Ref};
use std::collections::HashMap;
use std::rc::Rc;

fn main() {
    let args = pico_args::Arguments::from_env();
    let mut free = args.free().unwrap();
    let analysis_path = free.pop().unwrap();
    assert!(free.is_empty());

    let analysis: Analysis =
        serde_json::from_str(&std::fs::read_to_string(analysis_path).unwrap()).unwrap();

    let mut defs = Graph::<Rc<Def>, ()>::new();
    let mut def_index = HashMap::<u32, _>::new();

    let mut refs = Graph::<Rc<Def>, Ref>::new();

    for def in analysis.defs {
        let i = def.id.index;
        let def = Rc::new(def);
        let n = defs.add_node(def.clone());
        def_index.insert(i, n);
        assert!(refs.add_node(def) == n);
    }
}
