use microlp::{ComparisonOp, OptimizationDirection, Problem, SolveOptions};
use std::time::{Duration, Instant};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    let tl: u64 = a[2].parse().unwrap();
    let relax = a.len() > 3 && a[3] == "lp";
    let c = v["c"].as_array().unwrap(); let lb = v["lb"].as_array().unwrap();
    let ub = v["ub"].as_array().unwrap(); let it = v["int"].as_array().unwrap();
    let mut p = Problem::new(OptimizationDirection::Minimize);
    let mut vars = vec![]; let mut nint = 0;
    for i in 0..c.len() {
        let (l, u) = (lb[i].as_f64().unwrap(), ub[i].as_f64().unwrap());
        let u = if u > 1e29 { f64::INFINITY } else { u };
        if it[i].as_i64().unwrap() == 1 && !relax { nint += 1;
            vars.push(p.add_integer_var(c[i].as_f64().unwrap(), (l as i32, u as i32)));
        } else { vars.push(p.add_var(c[i].as_f64().unwrap(), (l, u))); }
    }
    let rows = v["rows"].as_array().unwrap();
    for r in rows {
        let idx = r[0].as_array().unwrap(); let co = r[1].as_array().unwrap();
        let e: Vec<_> = idx.iter().zip(co).map(|(i, x)| (vars[i.as_u64().unwrap() as usize], x.as_f64().unwrap())).collect();
        let (lo, hi) = (r[2].as_f64().unwrap(), r[3].as_f64().unwrap());
        if lo > -1e29 { p.add_constraint(e.as_slice(), ComparisonOp::Ge, lo); }
        if hi < 1e29 { p.add_constraint(e.as_slice(), ComparisonOp::Le, hi); }
    }
    println!("vars={} int={} rows={}", c.len(), nint, rows.len());
    let mut o = SolveOptions::default();
    o.time_limit = Some(Duration::from_secs(tl)); o.mip_gap = 1e-3;
    let t = Instant::now();
    match p.solve_with(o) {
        Ok(out) => { println!("debug: {:.300}", format!("{:?}", out.termination_reason()));
            let s = out.into_solution();
            match s { Ok(s) => println!("obj={} status={:?} time={:.1}s", s.objective(), s.status(), t.elapsed().as_secs_f64()),
                      Err(e) => println!("no solution: {:?} time={:.1}s", e, t.elapsed().as_secs_f64()) } }
        Err(e) => println!("err {:?} time={:.1}s", e, t.elapsed().as_secs_f64()),
    }
}
