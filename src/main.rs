use fxhash::{FxHashMap, FxHashSet};
use petgraph::{graphmap::GraphMap, Directed};
use rand::Rng;
use rls_data::{Analysis, Def, DefKind, RefKind};

fn main() {
    let args = pico_args::Arguments::from_env();
    let mut free = args.free().unwrap();
    let analysis_path = free.pop().unwrap();
    assert!(free.is_empty());

    let analysis: Analysis =
        serde_json::from_str(&std::fs::read_to_string(analysis_path).unwrap()).unwrap();

    let defs: FxHashMap<u32, Def> = analysis.defs.into_iter().map(|d| (d.id.index, d)).collect();

    let mut mod_hierarchy = GraphMap::<u32, (), Directed>::new();
    let mut filename_mod = FxHashMap::default();

    let mut root_id = None::<u32>;
    for (&id, def) in defs.iter() {
        if def.kind != DefKind::Mod {
            continue;
        }

        filename_mod.insert(def.span.file_name.clone(), id);

        if def.qualname == "::" {
            root_id = Some(id);
        }
        mod_hierarchy.add_node(id);
    }
    let root_id = root_id.expect("finding crate root");

    let mut edges = vec![];
    for id in mod_hierarchy.nodes() {
        let data = &defs[&id];
        for child in data.children.iter().map(|id| id.index) {
            if let Some(cdata) = defs.get(&child) {
                if cdata.kind == DefKind::Mod {
                    edges.push((id, child));
                }
            }
        }
    }
    for edge in edges {
        mod_hierarchy.add_edge(edge.0, edge.1, ());
    }

    let mut refs = GraphMap::<u32, u32, Directed>::new();

    for r in analysis.refs {
        if r.ref_id.krate != 0 {
            continue;
        }

        let to = if r.kind == RefKind::Mod {
            r.ref_id.index
        } else {
            if let Some(d) = defs.get(&r.ref_id.index) {
                filename_mod[&d.span.file_name]
            } else {
                continue;
            }
        };

        let from = filename_mod[&r.span.file_name];

        if from == to {
            continue;
        }

        refs.add_node(from);
        refs.add_node(to);

        if let Some(cnt) = refs.edge_weight_mut(from, to) {
            *cnt += 1;
        } else {
            refs.add_edge(
                from,
                to,
                if mod_hierarchy.contains_edge(from, to) {
                    0
                } else {
                    1
                },
            );
        }
    }
    let zero_edges = refs
        .all_edges()
        .filter(|(_, _, cnt)| **cnt == 0)
        .map(|(from, to, _)| (from, to))
        .collect::<Vec<_>>();
    for (from, to) in zero_edges {
        refs.remove_edge(from, to);
    }

    println!("digraph dependencies {{ graph [ rankdir = LR, ranksep = 20 ];");

    for m in mod_hierarchy.nodes() {
        println!("idx_{idx} [ label = \"{}\" ];", defs[&m].qualname, idx = m);
    }

    let mut rng = rand::thread_rng();

    for (from, to, cnt) in refs.all_edges() {
        let color = format!("/accent8/{}", rng.gen_range(1, 9));

        println!(
            "idx_{} -> idx_{} [ arrowhead = normal, weight = {cnt}, xlabel = \"{cnt}\", color = \"{color}\" ];",
            from,
            to,
            cnt = cnt,
            color = color,
        );
    }

    let mut remainings = mod_hierarchy.nodes().collect::<FxHashSet<u32>>();
    let mut stack = vec![root_id];

    while !remainings.is_empty() && !stack.is_empty() {
        print!("{{ rank = same; ");
        for i in std::mem::replace(&mut stack, vec![]) {
            remainings.remove(&i);
            for (_, to, ()) in mod_hierarchy.edges(i) {
                stack.push(to);
            }
            print!("idx_{}; ", i);
        }
        println!("}}");
    }

    println!("}}");
}
