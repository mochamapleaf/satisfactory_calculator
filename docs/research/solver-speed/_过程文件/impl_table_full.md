| run | plans / 105 | proven in 4 s | no plan | median gap to HiGHS-60s best | mean | >1 % | median time to first plan | median solve ms (proven) |
|---|---|---|---|---|---|---|---|---|
| impl_before4s | 104 | 43 | 1 | +1.00 % | +2.67 % | 31 | = end of solve | 26 |
| impl_after4s | 105 | 46 | 0 | +0.95 % | +2.64 % | 30 | 6 ms | 6 |

Per scenario (obj, ✔ = proven, ms):
| scenario | impl_before4s | impl_after4s |
|---|---|---|
| dsp_multi_p3_bld | 151.328 (4004 ms, 398 MW) | 151.327 (4000 ms, first 5 ms, 393 MW) |
| dsp_multi_p3_cmp | 232.007 (4004 ms, 398 MW) | 232.007 (4000 ms, first 9 ms, 398 MW) |
| dsp_multi_p3_res | ✔ 4944.74 (28 ms, 479 MW) | ✔ 4944.74 (4 ms, first 4 ms, 472 MW) |
| dsp_multi_pall_bld | 151.393 (4002 ms, 379 MW) | 151.392 (4000 ms, first 14 ms, 374 MW) |
| dsp_multi_pall_cmp | 233.256 (4004 ms, 366 MW) | 233.256 (4000 ms, first 32 ms, 366 MW) |
| dsp_multi_pall_res | ✔ 4944.74 (61 ms, 479 MW) | ✔ 4944.74 (9 ms, first 9 ms, 472 MW) |
| dsp_multi_plain_bld | 294.846 (4012 ms, 370 MW) | ✔ 294.846 (6 ms, first 2 ms, 369 MW) |
| dsp_multi_plain_cmp | 437.545 (4003 ms, 369 MW) | 437.545 (4000 ms, first 3 ms, 369 MW) |
| dsp_multi_plain_res | ✔ 11973.7 (13 ms, 484 MW) | ✔ 11973.7 (2 ms, first 2 ms, 482 MW) |
| dsp_um60_p3_bld | 115.039 (4005 ms, 288 MW) | 115.038 (4000 ms, first 5 ms, 284 MW) |
| dsp_um60_p3_cmp | 177.632 (4002 ms, 288 MW) | 177.632 (4000 ms, first 6 ms, 288 MW) |
| dsp_um60_p3_res | ✔ 3334.19 (21 ms, 341 MW) | ✔ 3334.19 (4 ms, first 4 ms, 337 MW) |
| dsp_um60_pall_bld | 114.086 (4004 ms, 274 MW) | 114.084 (4000 ms, first 15 ms, 271 MW) |
| dsp_um60_pall_cmp | 177.615 (4004 ms, 266 MW) | 175.486 (4000 ms, first 29 ms, 256 MW) |
| dsp_um60_pall_res | ✔ 3334.19 (44 ms, 341 MW) | ✔ 3334.19 (11 ms, first 11 ms, 337 MW) |
| dsp_um60_plain_bld | 220.115 (4008 ms, 278 MW) | ✔ 220.115 (6 ms, first 2 ms, 278 MW) |
| dsp_um60_plain_cmp | 329.406 (4006 ms, 274 MW) | 329.405 (4000 ms, first 3 ms, 274 MW) |
| dsp_um60_plain_res | ✔ 8506.16 (7 ms, 355 MW) | ✔ 8506.16 (2 ms, first 2 ms, 355 MW) |
| fa_sci6_mod_b4_bld | 243.196 (4003 ms, 152 MW) | 243.196 (4000 ms, first 22 ms, 152 MW) |
| fa_sci6_mod_b4_pow | 411.561 (4004 ms, 147 MW) | 411.561 (4001 ms, first 28 ms, 147 MW) |
| fa_sci6_mod_b4_res | 5839.24 (4005 ms, 257 MW) | 5839.24 (4000 ms, first 15 ms, 257 MW) |
| fa_sci6_mod_b8_bld | 264.282 (4005 ms, 150 MW) | 264.282 (4000 ms, first 10 ms, 150 MW) |
| fa_sci6_mod_b8_pow | 455.07 (4004 ms, 154 MW) | 445.174 (4000 ms, first 10 ms, 151 MW) |
| fa_sci6_mod_b8_res | 5840.8 (4006 ms, 292 MW) | 5840.8 (4002 ms, first 8 ms, 292 MW) |
| fa_sci6_mod_bld | 254.41 (4005 ms, 178 MW) | 253.453 (4000 ms, first 5 ms, 180 MW) |
| fa_sci6_mod_pow | 455.661 (4002 ms, 167 MW) | 455.204 (4000 ms, first 4 ms, 161 MW) |
| fa_sci6_mod_res | ✔ 5840.4 (20 ms, 410 MW) | ✔ 5840.4 (5 ms, first 5 ms, 410 MW) |
| fa_sci6_modall_bld | 254.41 (4004 ms, 178 MW) | 254.41 (4000 ms, first 6 ms, 178 MW) |
| fa_sci6_modall_pow | 455.038 (4004 ms, 164 MW) | 455.038 (4001 ms, first 9 ms, 164 MW) |
| fa_sci6_modall_res | ✔ 5840.4 (38 ms, 410 MW) | ✔ 5840.4 (6 ms, first 6 ms, 410 MW) |
| fa_sci6_plain_bld | 632.125 (4010 ms, 84 MW) | 631.743 (4000 ms, first 2 ms, 116 MW) |
| fa_sci6_plain_pow | 742.386 (4005 ms, 66 MW) | 742.386 (4000 ms, first 3 ms, 66 MW) |
| fa_sci6_plain_res | ✔ 16111.8 (15 ms, 148 MW) | ✔ 16111.8 (2 ms, first 2 ms, 148 MW) |
| fa_sci7_mod_b4_bld | 313.328 (4003 ms, 199 MW) | 313.328 (4000 ms, first 25 ms, 199 MW) |
| fa_sci7_mod_b4_pow | 538.441 (4004 ms, 193 MW) | 538.441 (4001 ms, first 48 ms, 193 MW) |
| fa_sci7_mod_b4_res | ✔ 7860.33 (154 ms, 337 MW) | ✔ 7860.33 (20 ms, first 20 ms, 337 MW) |
| fa_sci7_mod_b8_bld | 355.06 (4004 ms, 201 MW) | 353.917 (4000 ms, first 6 ms, 213 MW) |
| fa_sci7_mod_b8_pow | 605.214 (4002 ms, 205 MW) | 571.72 (4000 ms, first 16 ms, 194 MW) |
| fa_sci7_mod_b8_res | 7862.59 (4005 ms, 395 MW) | 7862.45 (4000 ms, first 8 ms, 482 MW) |
| fa_sci7_mod_bld | 337.188 (4004 ms, 238 MW) | 336.467 (4000 ms, first 6 ms, 240 MW) |
| fa_sci7_mod_pow | 602.199 (4004 ms, 220 MW) | 601.628 (4000 ms, first 6 ms, 206 MW) |
| fa_sci7_mod_res | ✔ 7862.06 (26 ms, 553 MW) | ✔ 7862.06 (5 ms, first 5 ms, 553 MW) |
| fa_sci7_modall_bld | 337.188 (4005 ms, 238 MW) | 336.516 (4000 ms, first 10 ms, 242 MW) |
| fa_sci7_modall_pow | 600.578 (4002 ms, 215 MW) | 600.578 (4000 ms, first 13 ms, 215 MW) |
| fa_sci7_modall_res | ✔ 7862.06 (49 ms, 553 MW) | ✔ 7862.06 (8 ms, first 8 ms, 553 MW) |
| fa_sci7_plain_bld | 956.073 (4008 ms, 214 MW) | 955.432 (4000 ms, first 2 ms, 168 MW) |
| fa_sci7_plain_pow | 1128.03 (4002 ms, 103 MW) | 1127.57 (4000 ms, first 3 ms, 103 MW) |
| fa_sci7_plain_res | ✔ 25490.5 (16 ms, 229 MW) | ✔ 25490.5 (2 ms, first 2 ms, 228 MW) |
| fx_sci6_mod_b4_bld | 202.649 (4005 ms, 170 MW) | 202.005 (4001 ms, first 60 ms, 171 MW) |
| fx_sci6_mod_b4_pow | 387.532 (4005 ms, 158 MW) | 387.532 (4008 ms, first 79 ms, 158 MW) |
| fx_sci6_mod_b4_res | 4677.73 (4005 ms, 270 MW) | 4677.73 (4002 ms, first 54 ms, 270 MW) |
| fx_sci6_mod_b8_bld | 201.738 (4004 ms, 153 MW) | 201.738 (4005 ms, first 20 ms, 153 MW) |
| fx_sci6_mod_b8_pow | 402.672 (4003 ms, 158 MW) | 395.26 (4001 ms, first 20 ms, 154 MW) |
| fx_sci6_mod_b8_res | 4679.25 (4005 ms, 555 MW) | 4679.13 (4001 ms, first 15 ms, 540 MW) |
| fx_sci6_mod_bld | 204.461 (4004 ms, 171 MW) | 204.461 (4000 ms, first 10 ms, 171 MW) |
| fx_sci6_mod_pow | 385.325 (4006 ms, 163 MW) | 385.353 (4000 ms, first 8 ms, 163 MW) |
| fx_sci6_mod_res | ✔ 4678.72 (47 ms, 716 MW) | ✔ 4678.72 (10 ms, first 10 ms, 716 MW) |
| fx_sci6_modall_bld | 204.461 (4003 ms, 171 MW) | 204.461 (4000 ms, first 16 ms, 171 MW) |
| fx_sci6_modall_pow | 361.818 (4004 ms, 125 MW) | 361.818 (4000 ms, first 20 ms, 125 MW) |
| fx_sci6_modall_res | ✔ 4678.72 (108 ms, 716 MW) | ✔ 4678.72 (15 ms, first 15 ms, 716 MW) |
| fx_sci6_plain_bld | 471.793 (4005 ms, 103 MW) | 471.488 (4000 ms, first 3 ms, 143 MW) |
| fx_sci6_plain_pow | 578.421 (4003 ms, 81 MW) | 578.421 (4000 ms, first 5 ms, 81 MW) |
| fx_sci6_plain_res | ✔ 11835.8 (24 ms, 149 MW) | ✔ 11835.8 (4 ms, first 4 ms, 148 MW) |
| sf_hmf7_alt_bld | 42.637 (4007 ms, 770 MW) | 42.637 (4000 ms, first 4 ms, 770 MW) |
| sf_hmf7_alt_pow | ✔ 642.955 (408 ms, 375 MW) | ✔ 642.955 (334 ms, first 10 ms, 375 MW) |
| sf_hmf7_alt_res | ✔ 1014.76 (26 ms, 1119 MW) | ✔ 1014.76 (22 ms, first 4 ms, 1119 MW) |
| sf_hmf7_std_bld | ✔ 225.056 (1 ms, 1585 MW) | ✔ 225.056 (1 ms, first 0 ms, 1585 MW) |
| sf_hmf7_std_pow | ✔ 2199.34 (6 ms, 1173 MW) | ✔ 2199.34 (5 ms, first 1 ms, 1173 MW) |
| sf_hmf7_std_res | ✔ 4057.86 (2 ms, 1585 MW) | ✔ 4057.86 (0 ms, first 0 ms, 1585 MW) |
| sf_pasta_alt_bld | 55.2754 (4006 ms, 2546 MW) | 55.2754 (4000 ms, first 5 ms, 2546 MW) |
| sf_pasta_alt_pow | 1747.14 (4002 ms, 1288 MW) | 1747.14 (4002 ms, first 13 ms, 1288 MW) |
| sf_pasta_alt_res | ✔ 2659.66 (29 ms, 3433 MW) | ✔ 2659.66 (8 ms, first 8 ms, 3433 MW) |
| sf_pasta_std_bld | ✔ 116.69 (11 ms, 2831 MW) | ✔ 116.69 (10 ms, first 2 ms, 2831 MW) |
| sf_pasta_std_pow | ✔ 2233.38 (64 ms, 1559 MW) | ✔ 2233.38 (72 ms, first 4 ms, 1559 MW) |
| sf_pasta_std_res | ✔ 6309.4 (8 ms, 2815 MW) | ✔ 6309.4 (2 ms, first 2 ms, 2815 MW) |
| sf_phase4_alt_bld | 304.762 (4017 ms, 8820 MW) | 304.762 (4000 ms, first 5 ms, 8820 MW) |
| sf_phase4_alt_pow | 7087.36 (4006 ms, 4497 MW) | 7087.36 (4000 ms, first 19 ms, 4497 MW) |
| sf_phase4_alt_res | ✔ 14554.5 (33 ms, 13444 MW) | ✔ 14554.5 (6 ms, first 6 ms, 13444 MW) |
| sf_phase4_oc250_bld | 157.462 (4005 ms, 11214 MW) | 157.462 (4000 ms, first 10 ms, 11214 MW) |
| sf_phase4_oc250_pow | 7087.36 (4003 ms, 4495 MW) | 7087.36 (4001 ms, first 26 ms, 4495 MW) |
| sf_phase4_oc250_res | ✔ 14554.5 (1154 ms, 13626 MW) | ✔ 14554.5 (1131 ms, first 9 ms, 13626 MW) |
| sf_phase4_sloop20_bld | no plan | 156.722 (4000 ms, first 91 ms, 7306 MW) |
| sf_phase4_sloop20_pow | 4138.74 (4005 ms, 2985 MW) | 4126.16 (4001 ms, first 279 ms, 2983 MW) |
| sf_phase4_sloop20_res | 5275.2 (4004 ms, 9314 MW) | 5724.85 (4002 ms, first 101 ms, 7559 MW) |
| sf_phase4_std_bld | ✔ 1181.74 (13 ms, 13723 MW) | ✔ 1181.74 (13 ms, first 3 ms, 13723 MW) |
| sf_phase4_std_pow | ✔ 15187 (111 ms, 8884 MW) | ✔ 15187 (91 ms, first 6 ms, 8884 MW) |
| sf_phase4_std_res | ✔ 50585.4 (11 ms, 13715 MW) | ✔ 50585.4 (3 ms, first 3 ms, 13715 MW) |
| sf_phase5_alt_bld | 157.811 (4006 ms, 8711 MW) | 157.811 (4000 ms, first 6 ms, 8711 MW) |
| sf_phase5_alt_pow | 6104.48 (4004 ms, 4723 MW) | 6104.48 (4000 ms, first 20 ms, 4723 MW) |
| sf_phase5_alt_res | ✔ 9903.86 (34 ms, 10736 MW) | ✔ 9903.86 (6 ms, first 6 ms, 10736 MW) |
| sf_phase5_std_bld | ✔ 427.228 (31 ms, 10486 MW) | ✔ 427.228 (18 ms, first 2 ms, 10486 MW) |
| sf_phase5_std_pow | 8766.46 (4006 ms, 6168 MW) | ✔ 8766.46 (3885 ms, first 6 ms, 6168 MW) |
| sf_phase5_std_res | ✔ 25231.8 (18 ms, 10486 MW) | ✔ 25231.8 (2 ms, first 2 ms, 10486 MW) |
| sf_rip10_alt_bld | ✔ 7.0702 (867 ms, 54 MW) | ✔ 7.0702 (720 ms, first 2 ms, 54 MW) |
| sf_rip10_alt_pow | ✔ 49.4627 (565 ms, 27 MW) | ✔ 49.4627 (454 ms, first 8 ms, 27 MW) |
| sf_rip10_alt_res | ✔ 37.1793 (16 ms, 68 MW) | ✔ 37.1793 (23 ms, first 4 ms, 68 MW) |
| sf_rip10_std_bld | ✔ 14.12 (0 ms, 78 MW) | ✔ 14.12 (0 ms, first 0 ms, 78 MW) |
| sf_rip10_std_pow | ✔ 109.636 (1 ms, 64 MW) | ✔ 109.636 (2 ms, first 0 ms, 64 MW) |
| sf_rip10_std_res | ✔ 120.14 (1 ms, 78 MW) | ✔ 120.14 (1 ms, first 1 ms, 78 MW) |
| sf_turbo_rip_alt_bld | 89.2388 (4007 ms, 1311 MW) | 87.5826 (4000 ms, first 5 ms, 1488 MW) |
| sf_turbo_rip_alt_pow | 1474.6 (4005 ms, 761 MW) | 1474.6 (4000 ms, first 14 ms, 761 MW) |
| sf_turbo_rip_alt_res | ✔ 5570.86 (32 ms, 3214 MW) | ✔ 5570.86 (6 ms, first 6 ms, 3214 MW) |
| sf_turbo_rip_std_bld | ✔ 319.555 (9 ms, 3596 MW) | ✔ 319.555 (8 ms, first 2 ms, 3596 MW) |
| sf_turbo_rip_std_pow | ✔ 4316.55 (86 ms, 2334 MW) | ✔ 4316.55 (70 ms, first 3 ms, 2334 MW) |
| sf_turbo_rip_std_res | ✔ 18632.2 (10 ms, 3560 MW) | ✔ 18632.2 (2 ms, first 2 ms, 3560 MW) |
