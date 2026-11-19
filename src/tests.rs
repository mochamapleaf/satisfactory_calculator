#[cfg(test)]
mod test{
    #[test]
    fn test_topological_sort(){
        const DEFAULT_JSON: &str = include_str!("../recipes/auto_output.json");
        let inst = crate::graph::Graph::from_str(DEFAULT_JSON);
        let mut all_resources: Vec<&str> = inst.topological_sort_result.keys().map(|s| s.as_str()).collect();
        all_resources.sort_unstable_by_key(|&s | inst.topological_sort_result[s].1);
        all_resources.reverse();
        println!("{:?}", all_resources);
    }
}

//TODO: Test on arm64 target before going to wasm32, for simpler debugging