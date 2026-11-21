"""Minimal flat 2D redraw of one game icon: smooth silhouette, 3-4 flat colour regions, shading removed.

Works at 4x resolution: k-means on Lab colour with lightness down-weighted (so lit and shaded faces of one
part merge), region boundaries smoothed by blurring each region's mask, then downsampled with antialiasing.
"""
import os
os.environ.setdefault("LOKY_MAX_CPU_COUNT", "8")  # skip loky's slow physical-core probe on Windows
os.environ.setdefault("OMP_NUM_THREADS", "1")  # tiny k-means per icon: threads only add overhead
import numpy as np
from PIL import Image
from scipy import ndimage
from skimage import color
from sklearn.cluster import KMeans

UP = 4


def redraw(img: Image.Image, k: int = 4, out_size: int = 64, lw: float = 0.55, smooth: float = 1.1, spread: float = 1.25, mono: bool = False) -> Image.Image:
    big = img.resize((img.width * UP, img.height * UP), Image.BICUBIC)
    a = np.asarray(big).astype(np.float32) / 255
    alpha = ndimage.gaussian_filter(a[..., 3], UP * 0.9)
    mask = alpha > 0.5
    if mask.sum() < 50:
        return img.resize((out_size, out_size), Image.LANCZOS)
    rgb = ndimage.gaussian_filter(a[..., :3], (UP * 0.8, UP * 0.8, 0))
    lab = color.rgb2lab(np.clip(rgb, 0, 1))
    feat = lab[mask] * np.array([lw, 1.0, 1.0])  # lightness matters less → shading merges
    kk = min(k, max(1, len(np.unique(np.round(feat / 6), axis=0))))
    km = KMeans(kk, n_init=3, random_state=0).fit(feat[:: max(1, len(feat) // 6000)])
    lab_lbl = np.full(mask.shape, -1)
    lab_lbl[mask] = km.predict(feat)
    # smooth region shapes: blur each one-hot layer, take argmax
    layers = np.stack([ndimage.gaussian_filter((lab_lbl == i).astype(np.float32), UP * smooth) for i in range(kk)])
    lbl = layers.argmax(0)
    # region colour: median Lab, lightness pulled toward the icon mean, a bit more chroma
    cols = []
    Lm = np.median(lab[mask][:, 0])
    for i in range(kk):
        sel = lab[mask & (lab_lbl == i)]
        c = np.median(sel, 0) if len(sel) else np.array([Lm, 0, 0])
        c = np.array([np.clip(Lm + spread * (c[0] - Lm), 8, 96), c[1] * 1.2, c[2] * 1.2])
        cols.append(c)
    cols = np.clip(color.lab2rgb(np.array(cols)[None])[0], 0, 1)
    out = np.zeros(a.shape, np.float32)
    if mono:  # one flat tint per icon (its dominant colour, mid lightness); detail only as lines
        # tint = the icon's most characteristic colour: region area weighted by its chroma, so a small
        # coloured part beats a large grey body; all-grey icons stay a cool neutral
        lab_c = color.rgb2lab(cols[None])[0]
        area = np.bincount(lbl[mask], minlength=kk)
        chroma = np.hypot(lab_c[:, 1], lab_c[:, 2])
        dom = lab_c[np.argmax(area * (chroma + 4))]
        c = np.hypot(dom[1], dom[2])
        scale = 0 if c < 8 else min(1.4, max(1.0, 38 / c))
        tint = [74, dom[1] * scale, dom[2] * scale] if scale else [76, -2, -6]
        cols = np.tile(np.clip(color.lab2rgb(np.array([[tint]], dtype=float))[0, 0], 0, 1), (kk, 1))
    out[..., :3] = cols[lbl]
    out[..., 3] = np.clip((alpha - 0.5) * 6 + 0.5, 0, 1)  # crisp but antialiased edge
    # thin strokes: silhouette edge and boundaries between regions, in a dark tone of the icon
    inside = alpha > 0.5
    edge = inside & ~ndimage.binary_erosion(inside, iterations=int(UP * 0.9))
    inner = np.zeros_like(inside)
    for i in range(kk):
        r = lbl == i
        inner |= r & ~ndimage.binary_erosion(r, iterations=int(UP * 0.45))
    inner &= ndimage.binary_erosion(inside, iterations=UP)
    ink = np.clip(color.lab2rgb(np.array([[[max(12, Lm * 0.35), 0, 0]]]))[0, 0], 0, 1)
    ink = 0.5 * ink + 0.5 * cols[np.bincount(lbl[inside], minlength=kk).argmax()] * 0.35
    out[edge, :3] = ink
    out[inner, :3] = out[inner, :3] * 0.45 + ink * 0.55
    im = Image.fromarray((out * 255).astype(np.uint8), "RGBA")
    return im.resize((out_size, out_size), Image.LANCZOS)
