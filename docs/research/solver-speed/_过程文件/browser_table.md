| run | plans | proven in 4 s | median gap to best known | mean | >1 % | first plan median / max (ms) | HiGHS load (ms) |
|---|---|---|---|---|---|---|---|
| browser_microlp4s | 105/105 | 45 | +0.95 % | +2.68 % | 30 | 10 / 329 | – |
| browser_highs4s | 105/105 | 63 | +0.06 % | +1.22 % | 14 | 8 / 341 | 65 |

browser_highs4s vs browser_microlp4s: better 41, worse 1 (|Δ| > 0.01 %)

| scenario | browser_microlp4s | browser_highs4s |
|---|---|---|
| dsp_multi_p3_bld | 151.327 (4003 ms) | 147.624 (4020 ms) -2.45 % |
| dsp_multi_p3_cmp | 232.007 (4007 ms) | 229.753 (4016 ms) -0.97 % |
| dsp_multi_pall_bld | 151.392 (4004 ms) | 150.252 (4021 ms) -0.75 % |
| dsp_multi_plain_cmp | 437.545 (4003 ms) | 437.511 ✔ (301 ms) -0.01 % |
| dsp_um60_p3_bld | 115.038 (4002 ms) | 111.053 (4016 ms) -3.46 % |
| dsp_um60_p3_cmp | 177.632 (4009 ms) | 171.246 (4015 ms) -3.59 % |
| dsp_um60_pall_bld | 114.084 (4003 ms) | 112.043 (4038 ms) -1.79 % |
| dsp_um60_pall_cmp | 175.486 (4005 ms) | 171.173 (4024 ms) -2.46 % |
| dsp_um60_plain_cmp | 329.406 (4004 ms) | 329.404 ✔ (170 ms) -0.00 % |
| fa_sci6_mod_b4_bld | 243.196 (4006 ms) | 227.031 (4020 ms) -6.65 % |
| fa_sci6_mod_b4_res | 5839.24 (4007 ms) | 5839.24 ✔ (1076 ms) +0.00 % |
| fa_sci6_mod_b8_bld | 264.282 (4005 ms) | 248.323 (4013 ms) -6.04 % |
| fa_sci6_mod_b8_pow | 445.174 (4005 ms) | 422.501 (4016 ms) -5.09 % |
| fa_sci6_mod_b8_res | 5840.8 (4004 ms) | 5840.42 ✔ (1923 ms) -0.01 % |
| fa_sci6_mod_bld | 254.41 (4002 ms) | 252.243 (4019 ms) -0.85 % |
| fa_sci6_mod_pow | 455.204 (4006 ms) | 451.202 (4020 ms) -0.88 % |
| fa_sci6_modall_bld | 254.41 (4003 ms) | 253.025 (4014 ms) -0.54 % |
| fa_sci6_modall_pow | 455.038 (4005 ms) | 448.782 (4025 ms) -1.37 % |
| fa_sci6_plain_bld | 631.743 (4002 ms) | 631.554 ✔ (1137 ms) -0.03 % |
| fa_sci6_plain_pow | 742.386 (4008 ms) | 741.549 ✔ (805 ms) -0.11 % |
| fa_sci7_mod_b4_bld | 313.328 (4003 ms) | 298.587 (4025 ms) -4.70 % |
| fa_sci7_mod_b8_bld | 353.917 (4009 ms) | 326.085 (4016 ms) -7.86 % |
| fa_sci7_mod_b8_pow | 571.72 (4006 ms) | 551.48 (4021 ms) -3.54 % |
| fa_sci7_mod_b8_res | 7862.45 (4006 ms) | 7862.3 ✔ (2490 ms) -0.00 % |
| fa_sci7_mod_bld | 336.467 (4003 ms) | 335.075 (4015 ms) -0.41 % |
| fa_sci7_mod_pow | 601.628 (4005 ms) | 596.89 (4023 ms) -0.79 % |
| fa_sci7_modall_bld | 336.516 (4003 ms) | 335.229 (4023 ms) -0.38 % |
| fa_sci7_modall_pow | 600.578 (4007 ms) | 592.887 (4025 ms) -1.28 % |
| fa_sci7_plain_bld | 955.432 (4002 ms) | 955.092 ✔ (1116 ms) -0.04 % |
| fa_sci7_plain_pow | 1127.57 (4008 ms) | 1125.22 ✔ (2107 ms) -0.21 % |
| fx_sci6_mod_b4_bld | 202.005 (4006 ms) | 188.485 (4039 ms) -6.69 % |
| fx_sci6_mod_b4_res | 4677.73 (4005 ms) | 4677.73 ✔ (1501 ms) +0.00 % |
| fx_sci6_mod_b8_bld | 201.738 (4005 ms) | 201.343 (4020 ms) -0.20 % |
| fx_sci6_mod_b8_pow | 395.26 (4005 ms) | 363.336 (4026 ms) -8.08 % |
| fx_sci6_mod_b8_res | 4679.13 (4005 ms) | 4678.62 ✔ (2559 ms) -0.01 % |
| fx_sci6_mod_bld | 204.461 (4002 ms) | 203.343 (4016 ms) -0.55 % |
| fx_sci6_mod_pow | 385.353 (4008 ms) | 382.603 (4018 ms) -0.71 % |
| fx_sci6_modall_bld | 204.461 (4003 ms) | 203.315 (4022 ms) -0.56 % |
| fx_sci6_modall_pow | 361.818 (4004 ms) | 356.772 (4029 ms) -1.39 % |
| fx_sci6_plain_bld | 471.488 (4002 ms) | 471.487 ✔ (1086 ms) -0.00 % |
| fx_sci6_plain_pow | 578.421 (4006 ms) | 577.746 ✔ (1151 ms) -0.12 % |
| sf_hmf7_alt_bld | 42.637 (4002 ms) | 42.586 (4014 ms) -0.12 % |
| sf_pasta_alt_bld | 55.2754 (4002 ms) | 52.1469 (4013 ms) -5.66 % |
| sf_pasta_alt_pow | 1747.14 (4005 ms) | 1747.14 ✔ (2731 ms) +0.00 % |
| sf_phase4_alt_bld | 304.762 (4004 ms) | 302.625 (4016 ms) -0.70 % |
| sf_phase4_alt_pow | 7087.36 (4006 ms) | 7087.3 ✔ (1606 ms) -0.00 % |
| sf_phase4_oc250_pow | 7087.42 (4004 ms) | 7087.3 ✔ (2095 ms) -0.00 % |
| sf_phase4_sloop20_bld | 156.722 (4005 ms) | 148.783 (4021 ms) -5.07 % |
| sf_phase4_sloop20_pow | 4138.37 (4004 ms) | 4123.93 (4044 ms) -0.35 % |
| sf_phase4_sloop20_res | 5724.85 (4004 ms) | 6239.2 (4205 ms) +8.98 % |
| sf_phase5_alt_bld | 157.811 (4004 ms) | 152.022 (4023 ms) -3.67 % |
| sf_phase5_std_pow | 8766.46 (4003 ms) | 8766.83 ✔ (1087 ms) +0.00 % |
| sf_turbo_rip_alt_bld | 89.2388 (4005 ms) | 85.4083 (4013 ms) -4.29 % |
| sf_turbo_rip_alt_pow | 1474.6 (4005 ms) | 1474.6 ✔ (3425 ms) +0.00 % |
