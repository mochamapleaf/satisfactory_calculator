| run | plans | proven in 4 s | median gap to best known | mean | >1 % | first plan median / max (ms) | HiGHS load (ms) |
|---|---|---|---|---|---|---|---|
| browser_microlp4s | 105/105 | 45 | +0.95 % | +2.55 % | 30 | 10 / 349 | – |
| browser_auto4s | 105/105 | 63 | +0.11 % | +1.01 % | 15 | 8 / 364 | 40, 52, 186 |

browser_auto4s vs browser_microlp4s: better 39, worse 0 (|Δ| > 0.01 %)

| scenario | browser_microlp4s | browser_auto4s |
|---|---|---|
| dsp_multi_p3_bld | 151.327 (4003 ms) | 147.624 (4021 ms) -2.45 % |
| dsp_multi_p3_cmp | 232.007 (4006 ms) | 229.753 (4017 ms) -0.97 % |
| dsp_multi_pall_bld | 151.392 (4003 ms) | 150.252 (4031 ms) -0.75 % |
| dsp_multi_plain_cmp | 437.545 (4002 ms) | 437.511 ✔ (311 ms) -0.01 % |
| dsp_um60_p3_bld | 115.038 (4003 ms) | 111.053 (4023 ms) -3.46 % |
| dsp_um60_p3_cmp | 177.632 (4010 ms) | 171.246 (4017 ms) -3.59 % |
| dsp_um60_pall_bld | 114.084 (4003 ms) | 112.765 (4029 ms) -1.16 % |
| dsp_um60_plain_cmp | 329.406 (4004 ms) | 329.404 ✔ (351 ms) -0.00 % |
| fa_sci6_mod_b4_bld | 243.196 (4006 ms) | 227.031 (4036 ms) -6.65 % |
| fa_sci6_mod_b4_res | 5839.24 (4005 ms) | 5839.24 ✔ (1140 ms) +0.00 % |
| fa_sci6_mod_b8_bld | 264.282 (4006 ms) | 248.323 (4016 ms) -6.04 % |
| fa_sci6_mod_b8_pow | 445.174 (4004 ms) | 422.892 (4019 ms) -5.01 % |
| fa_sci6_mod_b8_res | 5840.8 (4004 ms) | 5840.42 ✔ (1961 ms) -0.01 % |
| fa_sci6_mod_bld | 254.41 (4002 ms) | 252.243 (4020 ms) -0.85 % |
| fa_sci6_mod_pow | 455.204 (4006 ms) | 451.202 (4016 ms) -0.88 % |
| fa_sci6_modall_bld | 254.41 (4002 ms) | 253.025 (4035 ms) -0.54 % |
| fa_sci6_modall_pow | 455.038 (4006 ms) | 448.782 (4024 ms) -1.37 % |
| fa_sci6_plain_bld | 631.743 (4002 ms) | 631.554 ✔ (1139 ms) -0.03 % |
| fa_sci6_plain_pow | 742.386 (4008 ms) | 741.549 ✔ (824 ms) -0.11 % |
| fa_sci7_mod_b4_bld | 313.328 (4004 ms) | 300.599 (4039 ms) -4.06 % |
| fa_sci7_mod_b8_bld | 353.917 (4008 ms) | 326.085 (4043 ms) -7.86 % |
| fa_sci7_mod_b8_pow | 571.72 (4005 ms) | 551.48 (4022 ms) -3.54 % |
| fa_sci7_mod_b8_res | 7862.45 (4006 ms) | 7862.3 ✔ (2559 ms) -0.00 % |
| fa_sci7_mod_bld | 336.467 (4003 ms) | 335.075 (4022 ms) -0.41 % |
| fa_sci7_mod_pow | 601.628 (4005 ms) | 598.051 (4017 ms) -0.59 % |
| fa_sci7_modall_bld | 336.516 (4007 ms) | 335.229 (4035 ms) -0.38 % |
| fa_sci7_modall_pow | 600.578 (4005 ms) | 592.887 (4033 ms) -1.28 % |
| fa_sci7_plain_bld | 955.432 (4002 ms) | 955.092 ✔ (1114 ms) -0.04 % |
| fa_sci7_plain_pow | 1127.57 (4007 ms) | 1125.22 ✔ (2127 ms) -0.21 % |
| fx_sci6_mod_b4_bld | 202.005 (4005 ms) | 188.569 (4154 ms) -6.65 % |
| fx_sci6_mod_b4_res | 4677.73 (4006 ms) | 4677.73 ✔ (1613 ms) +0.00 % |
| fx_sci6_mod_b8_bld | 201.738 (4005 ms) | 201.343 (4134 ms) -0.20 % |
| fx_sci6_mod_b8_pow | 395.26 (4005 ms) | 363.336 (4038 ms) -8.08 % |
| fx_sci6_mod_b8_res | 4679.13 (4005 ms) | 4678.62 ✔ (2659 ms) -0.01 % |
| fx_sci6_mod_bld | 204.461 (4007 ms) | 203.343 (4025 ms) -0.55 % |
| fx_sci6_mod_pow | 385.353 (4009 ms) | 382.603 (4025 ms) -0.71 % |
| fx_sci6_modall_bld | 204.461 (4003 ms) | 203.315 (4034 ms) -0.56 % |
| fx_sci6_modall_pow | 361.818 (4004 ms) | 356.772 (4035 ms) -1.39 % |
| fx_sci6_plain_bld | 471.488 (4002 ms) | 471.487 ✔ (1124 ms) -0.00 % |
| fx_sci6_plain_pow | 578.421 (4006 ms) | 577.746 ✔ (1372 ms) -0.12 % |
| sf_hmf7_alt_bld | 42.637 (4003 ms) | 42.586 (4025 ms) -0.12 % |
| sf_pasta_alt_bld | 55.2754 (4002 ms) | 52.1469 (4026 ms) -5.66 % |
| sf_pasta_alt_pow | 1747.14 (4006 ms) | 1747.14 ✔ (2658 ms) +0.00 % |
| sf_phase4_alt_bld | 304.762 (4004 ms) | 302.625 (4019 ms) -0.70 % |
| sf_phase4_alt_pow | 7087.36 (4007 ms) | 7087.3 ✔ (1563 ms) -0.00 % |
| sf_phase4_oc250_pow | 7087.42 (4004 ms) | 7087.3 ✔ (2156 ms) -0.00 % |
| sf_phase4_sloop20_bld | 156.722 (4004 ms) | 148.783 (4029 ms) -5.07 % |
| sf_phase5_alt_bld | 157.811 (4006 ms) | 152.022 (4021 ms) -3.67 % |
| sf_phase5_std_pow | 8766.46 (4002 ms) | 8766.83 ✔ (1091 ms) +0.00 % |
| sf_turbo_rip_alt_bld | 89.2388 (4002 ms) | 85.4083 (4026 ms) -4.29 % |
| sf_turbo_rip_alt_pow | 1474.6 (4005 ms) | 1474.6 ✔ (3313 ms) +0.00 % |
