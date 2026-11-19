//! Turn a solved plan into graph edges: for every item, split each producer's output
//! across the consumers in proportion to their demand.
//!
//! Node ids: `r:<recipe id>`, `raw:<item>`, `sup:<item>`, `out:<item>` (target), `extra:<item>` (surplus).
use super::{Flow, Plan};
use std::collections::BTreeMap;

pub fn allocate(plan: &Plan) -> Vec<Flow> {
    let mut prod: BTreeMap<&str, Vec<(String, f64)>> = BTreeMap::new();
    let mut cons: BTreeMap<&str, Vec<(String, f64)>> = BTreeMap::new();
    let mut names: BTreeMap<&str, &str> = BTreeMap::new();
    for r in &plan.recipes {
        for o in &r.outputs {
            prod.entry(&o.item).or_default().push((format!("r:{}", r.id), o.rate));
            names.insert(&o.item, &o.name);
        }
        for i in &r.inputs {
            cons.entry(&i.item).or_default().push((format!("r:{}", r.id), i.rate));
            names.insert(&i.item, &i.name);
        }
    }
    for l in &plan.raw {
        prod.entry(&l.item).or_default().push((format!("raw:{}", l.item), l.rate));
        names.insert(&l.item, &l.name);
    }
    for l in &plan.supplied {
        prod.entry(&l.item).or_default().push((format!("sup:{}", l.item), l.rate));
        names.insert(&l.item, &l.name);
    }
    for l in &plan.targets {
        cons.entry(&l.item).or_default().push((format!("out:{}", l.item), l.rate));
        names.insert(&l.item, &l.name);
    }
    for l in &plan.surplus {
        cons.entry(&l.item).or_default().push((format!("extra:{}", l.item), l.rate));
        names.insert(&l.item, &l.name);
    }
    let mut flows = Vec::new();
    for (item, ps) in &prod {
        let Some(cs) = cons.get(item) else { continue };
        let total_c: f64 = cs.iter().map(|c| c.1).sum();
        if total_c <= 0.0 {
            continue;
        }
        let total_p: f64 = ps.iter().map(|p| p.1).sum();
        let scale = total_c.min(total_p);
        for (p, pr) in ps {
            for (c, cr) in cs {
                if p == c {
                    continue; // self-loop of a recipe consuming its own product
                }
                let rate = scale * (pr / total_p) * (cr / total_c);
                if rate > 1e-6 {
                    flows.push(super::Flow {
                        from: p.clone(),
                        to: c.clone(),
                        item: item.to_string(),
                        name: names[item].to_string(),
                        rate,
                    });
                }
            }
        }
    }
    flows
}
