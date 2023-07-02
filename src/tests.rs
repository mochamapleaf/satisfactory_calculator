#[cfg(test)]
mod test{
    use crate::graph::*;
    #[test]
    fn test_topological_sort(){
        let mut inst = crate::graph::Graph::new("recipes/auto_output.json");
        let mut all_resources: Vec<&str> = inst.topological_sort_result.keys().map(|s| s.as_str()).collect();
        all_resources.sort_unstable_by_key(|&s | inst.topological_sort_result[s].1);
        all_resources.reverse();
        println!("{:?}", all_resources);
    }
}

//TODO: Test on arm64 target before going to wasm32, for simpler debugging