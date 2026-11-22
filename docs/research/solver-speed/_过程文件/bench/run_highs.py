"""Solve dumped .lp models with native HiGHS (highspy): time to optimal at the same 1e-4 gap.
usage: python run_highs.py <dir> [time_limit_s] [threads]"""
import sys, os, json, time, glob
import highspy

d = sys.argv[1]
tl = float(sys.argv[2]) if len(sys.argv) > 2 else 60.0
threads = int(sys.argv[3]) if len(sys.argv) > 3 else 1
for f in sorted(glob.glob(os.path.join(d, '*.lp'))):
    h = highspy.Highs()
    h.setOptionValue('output_flag', False)
    h.setOptionValue('threads', threads)
    h.setOptionValue('time_limit', tl)
    h.setOptionValue('mip_rel_gap', 1e-4)
    h.readModel(f)
    t0 = time.perf_counter()
    h.run()
    dt = time.perf_counter() - t0
    info = h.getInfo()
    st = h.modelStatusToString(h.getModelStatus())
    print(json.dumps({
        'name': os.path.basename(f)[:-3], 'highs_s': round(dt, 3), 'status': st,
        'obj': info.objective_function_value, 'bound': info.mip_dual_bound,
        'gap': info.mip_gap, 'nodes': info.mip_node_count,
        'rows': h.getNumRow(), 'cols': h.getNumCol(),
    }), flush=True)
