<script lang="ts">
	import { onMount } from 'svelte';
	import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';

	const REFERENCE_WIDTH = 400;
	const REFERENCE_HEIGHT = 744;
	const REFERENCE_RATIO = REFERENCE_WIDTH / REFERENCE_HEIGHT;

	type Tab = 'selector' | 'history';
	type ModelName =
		| 'HSL'
		| 'HSV'
		| 'OKLCH'
		| 'RGB'
		| 'LCH'
		| 'OKLAB'
		| 'P3'
		| 'HEX';

	type RGB = {
		r: number;
		g: number;
		b: number;
	};

	type Control = {
		key: string;
		label: string;
		min: number;
		max: number;
		step: number;
		value: number;
		suffix?: string;
	};

	type HistoryColor = {
		hex: string;
		name: string;
		createdAt: number;
	};

	const models: ModelName[] = [
		'HSL',
		'HSV',
		'OKLCH',
		'RGB',
		'LCH',
		'OKLAB',
		'P3',
		'HEX'
	];

	let activeTab: Tab = 'selector';
	let activeModel: ModelName = 'HSL';

	let rgb: RGB = {
		r: 217,
		g: 217,
		b: 217
	};

	let uiScale = 0.8;

	let controls: Control[] = [];

	let hexInput = '#D9D9D9';

	let toastVisible = false;
	let toastText = '';
	let toastTimer: ReturnType<typeof setTimeout> | undefined;

	let history: HistoryColor[] = [];
	let historyView: 'list' | 'grid' = 'list';

	let resizeSettleTimer: ReturnType<typeof setTimeout> | null = null;

	const clamp = (value: number, min: number, max: number) =>
		Math.min(max, Math.max(min, value));

	const round = (value: number, decimals = 2) => {
		const factor = 10 ** decimals;
		return Math.round(value * factor) / factor;
	};

	const normalizeHue = (h: number) => ((h % 360) + 360) % 360;

	function componentToHex(value: number) {
		return Math.round(clamp(value, 0, 255))
			.toString(16)
			.padStart(2, '0')
			.toUpperCase();
	}

	function rgbToHex(color: RGB) {
		return `#${componentToHex(color.r)}${componentToHex(color.g)}${componentToHex(color.b)}`;
	}

	function hexToRgb(hex: string): RGB | null {
		const cleaned = hex.trim().replace('#', '');

		if (!/^[0-9a-fA-F]{6}$/.test(cleaned)) {
			return null;
		}

		return {
			r: parseInt(cleaned.slice(0, 2), 16),
			g: parseInt(cleaned.slice(2, 4), 16),
			b: parseInt(cleaned.slice(4, 6), 16)
		};
	}

	// 142 named reference colors
	// (CSS Color Module Level 4 extended keyword list, expanded for finer nearest-name matching)
	const NAMED_COLORS: { name: string; r: number; g: number; b: number }[] = [
		{ name: 'Alice Blue', r: 240, g: 248, b: 255 },
		{ name: 'Antique White', r: 250, g: 235, b: 215 },
		{ name: 'Aqua', r: 0, g: 255, b: 255 },
		{ name: 'Aquamarine', r: 127, g: 255, b: 212 },
		{ name: 'Azure', r: 240, g: 255, b: 255 },
		{ name: 'Beige', r: 245, g: 245, b: 220 },
		{ name: 'Bisque', r: 255, g: 228, b: 196 },
		{ name: 'Black', r: 0, g: 0, b: 0 },
		{ name: 'Blanched Almond', r: 255, g: 235, b: 205 },
		{ name: 'Blue', r: 0, g: 0, b: 255 },
		{ name: 'Blue Violet', r: 138, g: 43, b: 226 },
		{ name: 'Brown', r: 165, g: 42, b: 42 },
		{ name: 'Burlywood', r: 222, g: 184, b: 135 },
		{ name: 'Cadet Blue', r: 95, g: 158, b: 160 },
		{ name: 'Charcoal', r: 54, g: 69, b: 79 },
		{ name: 'Chartreuse', r: 127, g: 255, b: 0 },
		{ name: 'Chocolate', r: 210, g: 105, b: 30 },
		{ name: 'Coral', r: 255, g: 127, b: 80 },
		{ name: 'Cornflower Blue', r: 100, g: 149, b: 237 },
		{ name: 'Cornsilk', r: 255, g: 248, b: 220 },
		{ name: 'Crimson', r: 220, g: 20, b: 60 },
		{ name: 'Cyan', r: 0, g: 255, b: 255 },
		{ name: 'Dark Blue', r: 0, g: 0, b: 139 },
		{ name: 'Dark Cyan', r: 0, g: 139, b: 139 },
		{ name: 'Dark Goldenrod', r: 184, g: 134, b: 11 },
		{ name: 'Dark Gray', r: 169, g: 169, b: 169 },
		{ name: 'Dark Green', r: 0, g: 100, b: 0 },
		{ name: 'Dark Khaki', r: 189, g: 183, b: 107 },
		{ name: 'Dark Magenta', r: 139, g: 0, b: 139 },
		{ name: 'Dark Olive Green', r: 85, g: 107, b: 47 },
		{ name: 'Dark Orange', r: 255, g: 140, b: 0 },
		{ name: 'Dark Orchid', r: 153, g: 50, b: 204 },
		{ name: 'Dark Red', r: 139, g: 0, b: 0 },
		{ name: 'Dark Salmon', r: 233, g: 150, b: 122 },
		{ name: 'Dark Sea Green', r: 143, g: 188, b: 143 },
		{ name: 'Dark Slate Blue', r: 72, g: 61, b: 139 },
		{ name: 'Dark Slate Gray', r: 47, g: 79, b: 79 },
		{ name: 'Dark Turquoise', r: 0, g: 206, b: 209 },
		{ name: 'Dark Violet', r: 148, g: 0, b: 211 },
		{ name: 'Deep Pink', r: 255, g: 20, b: 147 },
		{ name: 'Deep Sky Blue', r: 0, g: 191, b: 255 },
		{ name: 'Dim Gray', r: 105, g: 105, b: 105 },
		{ name: 'Dodger Blue', r: 30, g: 144, b: 255 },
		{ name: 'Firebrick', r: 178, g: 34, b: 34 },
		{ name: 'Floral White', r: 255, g: 250, b: 240 },
		{ name: 'Forest Green', r: 34, g: 139, b: 34 },
		{ name: 'Fuchsia', r: 255, g: 0, b: 255 },
		{ name: 'Gainsboro', r: 220, g: 220, b: 220 },
		{ name: 'Ghost White', r: 248, g: 248, b: 255 },
		{ name: 'Gold', r: 255, g: 215, b: 0 },
		{ name: 'Goldenrod', r: 218, g: 165, b: 32 },
		{ name: 'Gray', r: 128, g: 128, b: 128 },
		{ name: 'Green', r: 0, g: 128, b: 0 },
		{ name: 'Green Yellow', r: 173, g: 255, b: 47 },
		{ name: 'Honeydew', r: 240, g: 255, b: 240 },
		{ name: 'Hot Pink', r: 255, g: 105, b: 180 },
		{ name: 'Indian Red', r: 205, g: 92, b: 92 },
		{ name: 'Indigo', r: 75, g: 0, b: 130 },
		{ name: 'Ivory', r: 255, g: 255, b: 240 },
		{ name: 'Khaki', r: 240, g: 230, b: 140 },
		{ name: 'Lavender', r: 230, g: 230, b: 250 },
		{ name: 'Lavender Blush', r: 255, g: 240, b: 245 },
		{ name: 'Lawn Green', r: 124, g: 252, b: 0 },
		{ name: 'Lemon Chiffon', r: 255, g: 250, b: 205 },
		{ name: 'Light Blue', r: 173, g: 216, b: 230 },
		{ name: 'Light Coral', r: 240, g: 128, b: 128 },
		{ name: 'Light Cyan', r: 224, g: 255, b: 255 },
		{ name: 'Light Goldenrod Yellow', r: 250, g: 250, b: 210 },
		{ name: 'Light Gray', r: 211, g: 211, b: 211 },
		{ name: 'Light Green', r: 144, g: 238, b: 144 },
		{ name: 'Light Pink', r: 255, g: 182, b: 193 },
		{ name: 'Light Salmon', r: 255, g: 160, b: 122 },
		{ name: 'Light Sea Green', r: 32, g: 178, b: 170 },
		{ name: 'Light Sky Blue', r: 135, g: 206, b: 250 },
		{ name: 'Light Slate Gray', r: 119, g: 136, b: 153 },
		{ name: 'Light Steel Blue', r: 176, g: 196, b: 222 },
		{ name: 'Light Yellow', r: 255, g: 255, b: 224 },
		{ name: 'Lime', r: 0, g: 255, b: 0 },
		{ name: 'Lime Green', r: 50, g: 205, b: 50 },
		{ name: 'Linen', r: 250, g: 240, b: 230 },
		{ name: 'Magenta', r: 255, g: 0, b: 255 },
		{ name: 'Maroon', r: 128, g: 0, b: 0 },
		{ name: 'Medium Aquamarine', r: 102, g: 205, b: 170 },
		{ name: 'Medium Blue', r: 0, g: 0, b: 205 },
		{ name: 'Medium Orchid', r: 186, g: 85, b: 211 },
		{ name: 'Medium Purple', r: 147, g: 112, b: 219 },
		{ name: 'Medium Sea Green', r: 60, g: 179, b: 113 },
		{ name: 'Medium Slate Blue', r: 123, g: 104, b: 238 },
		{ name: 'Medium Spring Green', r: 0, g: 250, b: 154 },
		{ name: 'Medium Turquoise', r: 72, g: 209, b: 204 },
		{ name: 'Medium Violet Red', r: 199, g: 21, b: 133 },
		{ name: 'Midnight Blue', r: 25, g: 25, b: 112 },
		{ name: 'Mint Cream', r: 245, g: 255, b: 250 },
		{ name: 'Misty Rose', r: 255, g: 228, b: 225 },
		{ name: 'Moccasin', r: 255, g: 228, b: 181 },
		{ name: 'Navajo White', r: 255, g: 222, b: 173 },
		{ name: 'Navy', r: 0, g: 0, b: 128 },
		{ name: 'Old Lace', r: 253, g: 245, b: 230 },
		{ name: 'Olive', r: 128, g: 128, b: 0 },
		{ name: 'Olive Drab', r: 107, g: 142, b: 35 },
		{ name: 'Orange', r: 255, g: 165, b: 0 },
		{ name: 'Orange Red', r: 255, g: 69, b: 0 },
		{ name: 'Orchid', r: 218, g: 112, b: 214 },
		{ name: 'Pale Goldenrod', r: 238, g: 232, b: 170 },
		{ name: 'Pale Green', r: 152, g: 251, b: 152 },
		{ name: 'Pale Turquoise', r: 175, g: 238, b: 238 },
		{ name: 'Pale Violet Red', r: 219, g: 112, b: 147 },
		{ name: 'Papaya Whip', r: 255, g: 239, b: 213 },
		{ name: 'Peach Puff', r: 255, g: 218, b: 185 },
		{ name: 'Peru', r: 205, g: 133, b: 63 },
		{ name: 'Pink', r: 255, g: 192, b: 203 },
		{ name: 'Plum', r: 221, g: 160, b: 221 },
		{ name: 'Powder Blue', r: 176, g: 224, b: 230 },
		{ name: 'Purple', r: 128, g: 0, b: 128 },
		{ name: 'Rebecca Purple', r: 102, g: 51, b: 153 },
		{ name: 'Red', r: 255, g: 0, b: 0 },
		{ name: 'Rosy Brown', r: 188, g: 143, b: 143 },
		{ name: 'Royal Blue', r: 65, g: 105, b: 225 },
		{ name: 'Saddle Brown', r: 139, g: 69, b: 19 },
		{ name: 'Salmon', r: 250, g: 128, b: 114 },
		{ name: 'Sandy Brown', r: 244, g: 164, b: 96 },
		{ name: 'Sea Green', r: 46, g: 139, b: 87 },
		{ name: 'Seashell', r: 255, g: 245, b: 238 },
		{ name: 'Sienna', r: 160, g: 82, b: 45 },
		{ name: 'Silver', r: 192, g: 192, b: 192 },
		{ name: 'Sky Blue', r: 135, g: 206, b: 235 },
		{ name: 'Slate Blue', r: 106, g: 90, b: 205 },
		{ name: 'Slate Gray', r: 112, g: 128, b: 144 },
		{ name: 'Snow', r: 255, g: 250, b: 250 },
		{ name: 'Spring Green', r: 0, g: 255, b: 127 },
		{ name: 'Steel Blue', r: 70, g: 130, b: 180 },
		{ name: 'Tan', r: 210, g: 180, b: 140 },
		{ name: 'Teal', r: 0, g: 128, b: 128 },
		{ name: 'Thistle', r: 216, g: 191, b: 216 },
		{ name: 'Tomato', r: 255, g: 99, b: 71 },
		{ name: 'Turquoise', r: 64, g: 224, b: 208 },
		{ name: 'Violet', r: 238, g: 130, b: 238 },
		{ name: 'Wheat', r: 245, g: 222, b: 179 },
		{ name: 'White', r: 255, g: 255, b: 255 },
		{ name: 'White Smoke', r: 245, g: 245, b: 245 },
		{ name: 'Yellow', r: 255, g: 255, b: 0 },
		{ name: 'Yellow Green', r: 154, g: 205, b: 50 }
	];

	function nearestColorName(color: RGB) {
		let best = NAMED_COLORS[0];
		let bestDistance = Infinity;

		for (const candidate of NAMED_COLORS) {
			const dr = color.r - candidate.r;
			const dg = color.g - candidate.g;
			const db = color.b - candidate.b;

			const distance = dr * dr + dg * dg + db * db;

			if (distance < bestDistance) {
				bestDistance = distance;
				best = candidate;
			}
		}

		return best.name;
	}

	function srgbToLinear(value: number) {
		const v = value / 255;
		return v <= 0.04045
			? v / 12.92
			: Math.pow((v + 0.055) / 1.055, 2.4);
	}

	function linearToSrgb(value: number) {
		const v =
			value <= 0.0031308
				? 12.92 * value
				: 1.055 * Math.pow(Math.max(value, 0), 1 / 2.4) - 0.055;

		return clamp(v * 255, 0, 255);
	}

	function rgbToXyz(color: RGB) {
		const r = srgbToLinear(color.r);
		const g = srgbToLinear(color.g);
		const b = srgbToLinear(color.b);

		return {
			x: r * 0.4124564 + g * 0.3575761 + b * 0.1804375,
			y: r * 0.2126729 + g * 0.7151522 + b * 0.072175,
			z: r * 0.0193339 + g * 0.119192 + b * 0.9503041
		};
	}

	function xyzToRgb(x: number, y: number, z: number): RGB {
		const r =
			x * 3.2404542 +
			y * -1.5371385 +
			z * -0.4985314;

		const g =
			x * -0.969266 +
			y * 1.8760108 +
			z * 0.041556;

		const b =
			x * 0.0556434 +
			y * -0.2040259 +
			z * 1.0572252;

		return {
			r: linearToSrgb(r),
			g: linearToSrgb(g),
			b: linearToSrgb(b)
		};
	}

	function rgbToHsl(color: RGB) {
		let r = color.r / 255;
		let g = color.g / 255;
		let b = color.b / 255;

		const max = Math.max(r, g, b);
		const min = Math.min(r, g, b);

		let h = 0;
		let s = 0;

		const l = (max + min) / 2;
		const d = max - min;

		if (d !== 0) {
			s = d / (1 - Math.abs(2 * l - 1));

			switch (max) {
				case r:
					h = 60 * (((g - b) / d) % 6);
					break;

				case g:
					h = 60 * ((b - r) / d + 2);
					break;

				case b:
					h = 60 * ((r - g) / d + 4);
					break;
			}
		}

		return {
			h: normalizeHue(h),
			s: s * 100,
			l: l * 100
		};
	}

	function hslToRgb(h: number, s: number, l: number): RGB {
		h = normalizeHue(h);
		s = clamp(s, 0, 100) / 100;
		l = clamp(l, 0, 100) / 100;

		const c = (1 - Math.abs(2 * l - 1)) * s;
		const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
		const m = l - c / 2;

		let r = 0;
		let g = 0;
		let b = 0;

		if (h < 60) {
			r = c;
			g = x;
		} else if (h < 120) {
			r = x;
			g = c;
		} else if (h < 180) {
			g = c;
			b = x;
		} else if (h < 240) {
			g = x;
			b = c;
		} else if (h < 300) {
			r = x;
			b = c;
		} else {
			r = c;
			b = x;
		}

		return {
			r: (r + m) * 255,
			g: (g + m) * 255,
			b: (b + m) * 255
		};
	}

	function rgbToHsv(color: RGB) {
		const r = color.r / 255;
		const g = color.g / 255;
		const b = color.b / 255;

		const max = Math.max(r, g, b);
		const min = Math.min(r, g, b);
		const d = max - min;

		let h = 0;

		if (d !== 0) {
			switch (max) {
				case r:
					h = 60 * (((g - b) / d) % 6);
					break;
				case g:
					h = 60 * ((b - r) / d + 2);
					break;
				case b:
					h = 60 * ((r - g) / d + 4);
					break;
			}
		}

		return {
			h: normalizeHue(h),
			s: max === 0 ? 0 : (d / max) * 100,
			v: max * 100
		};
	}

	function hsvToRgb(h: number, s: number, v: number): RGB {
		h = normalizeHue(h);
		s = clamp(s, 0, 100) / 100;
		v = clamp(v, 0, 100) / 100;

		const c = v * s;
		const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
		const m = v - c;

		let r = 0;
		let g = 0;
		let b = 0;

		if (h < 60) {
			r = c;
			g = x;
		} else if (h < 120) {
			r = x;
			g = c;
		} else if (h < 180) {
			g = c;
			b = x;
		} else if (h < 240) {
			g = x;
			b = c;
		} else if (h < 300) {
			r = x;
			b = c;
		} else {
			r = c;
			b = x;
		}

		return {
			r: (r + m) * 255,
			g: (g + m) * 255,
			b: (b + m) * 255
		};
	}

	function xyzToLab(x: number, y: number, z: number) {
		const xn = 0.95047;
		const yn = 1;
		const zn = 1.08883;

		const epsilon = 216 / 24389;
		const kappa = 24389 / 27;

		const f = (t: number) =>
			t > epsilon
				? Math.cbrt(t)
				: (kappa * t + 16) / 116;

		const fx = f(x / xn);
		const fy = f(y / yn);
		const fz = f(z / zn);

		return {
			l: 116 * fy - 16,
			a: 500 * (fx - fy),
			b: 200 * (fy - fz)
		};
	}

	function labToXyz(l: number, a: number, b: number) {
		const xn = 0.95047;
		const yn = 1;
		const zn = 1.08883;

		const epsilon = 216 / 24389;
		const kappa = 24389 / 27;

		const fy = (l + 16) / 116;
		const fx = fy + a / 500;
		const fz = fy - b / 200;

		const inverse = (t: number) => {
			const cube = t ** 3;

			return cube > epsilon
				? cube
				: (116 * t - 16) / kappa;
		};

		return {
			x: xn * inverse(fx),
			y: yn * inverse(fy),
			z: zn * inverse(fz)
		};
	}

	function rgbToLch(color: RGB) {
		const xyz = rgbToXyz(color);
		const lab = xyzToLab(xyz.x, xyz.y, xyz.z);

		const c = Math.sqrt(lab.a ** 2 + lab.b ** 2);
		const h = normalizeHue(
			Math.atan2(lab.b, lab.a) * (180 / Math.PI)
		);

		return {
			l: lab.l,
			c,
			h
		};
	}

	function lchToRgb(l: number, c: number, h: number): RGB {
		const radians = normalizeHue(h) * (Math.PI / 180);

		const a = c * Math.cos(radians);
		const b = c * Math.sin(radians);

		const xyz = labToXyz(l, a, b);

		return xyzToRgb(xyz.x, xyz.y, xyz.z);
	}

	function xyzToOklab(x: number, y: number, z: number) {
		const l =
			0.819022437996703 * x +
			0.3619062600528904 * y -
			0.1288737815209879 * z;

		const m =
			0.0329836539323885 * x +
			0.9292868615863434 * y +
			0.0361446663506424 * z;

		const s =
			0.0481771893596242 * x +
			0.2642395317527308 * y +
			0.6335478284694309 * z;

		const l_ = Math.cbrt(l);
		const m_ = Math.cbrt(m);
		const s_ = Math.cbrt(s);

		return {
			l:
				0.2104542553 * l_ +
				0.793617785 * m_ -
				0.0040720468 * s_,

			a:
				1.9779984951 * l_ -
				2.428592205 * m_ +
				0.4505937099 * s_,

			b:
				0.0259040371 * l_ +
				0.7827717662 * m_ -
				0.808675766 * s_
		};
	}

	function oklabToXyz(l: number, a: number, b: number) {
		const l_ =
			l +
			0.3963377774 * a +
			0.2158037573 * b;

		const m_ =
			l -
			0.1055613458 * a -
			0.0638541728 * b;

		const s_ =
			l -
			0.0894841775 * a -
			1.291485548 * b;

		const ll = l_ ** 3;
		const mm = m_ ** 3;
		const ss = s_ ** 3;

		return {
			x:
				1.2268798734 * ll -
				0.5578149966 * mm -
				0.2813910502 * ss,

			y:
				-0.0405757626 * ll +
				1.1122868294 * mm -
				0.0717110667 * ss,

			z:
				-0.0763729497 * ll -
				0.4214933239 * mm +
				1.5869240244 * ss
		};
	}

	function rgbToOklab(color: RGB) {
		const xyz = rgbToXyz(color);
		return xyzToOklab(xyz.x, xyz.y, xyz.z);
	}

	function oklabToRgb(l: number, a: number, b: number): RGB {
		const xyz = oklabToXyz(l, a, b);
		return xyzToRgb(xyz.x, xyz.y, xyz.z);
	}

	function rgbToOklch(color: RGB) {
		const lab = rgbToOklab(color);

		return {
			l: lab.l,
			c: Math.sqrt(lab.a ** 2 + lab.b ** 2),
			h: normalizeHue(
				Math.atan2(lab.b, lab.a) * (180 / Math.PI)
			)
		};
	}

	function oklchToRgb(l: number, c: number, h: number): RGB {
		const radians = normalizeHue(h) * (Math.PI / 180);

		const a = c * Math.cos(radians);
		const b = c * Math.sin(radians);

		return oklabToRgb(l, a, b);
	}

	function linearP3ToXyz(r: number, g: number, b: number) {
		return {
			x:
				r * 0.4865709486 +
				g * 0.2656676932 +
				b * 0.1982172852,

			y:
				r * 0.2289745641 +
				g * 0.6917385218 +
				b * 0.0792869141,

			z:
				g * 0.0451133819 +
				b * 1.0439443689
		};
	}

	function xyzToLinearP3(x: number, y: number, z: number) {
		return {
			r:
				x * 2.4934969119 -
				y * 0.9313836179 -
				z * 0.4027107845,

			g:
				x * -0.8294889696 +
				y * 1.7626640603 +
				z * 0.0236246858,

			b:
				x * 0.03584583 -
				y * 0.0761723893 +
				z * 0.956884524
		};
	}

	function encodeRgb01(value: number) {
		return value <= 0.0031308
			? 12.92 * value
			: 1.055 * Math.pow(Math.max(value, 0), 1 / 2.4) - 0.055;
	}

	function decodeRgb01(value: number) {
		return value <= 0.04045
			? value / 12.92
			: Math.pow((value + 0.055) / 1.055, 2.4);
	}

	function rgbToP3(color: RGB) {
		const xyz = rgbToXyz(color);
		const p3 = xyzToLinearP3(xyz.x, xyz.y, xyz.z);

		return {
			r: clamp(encodeRgb01(p3.r) * 100, 0, 100),
			g: clamp(encodeRgb01(p3.g) * 100, 0, 100),
			b: clamp(encodeRgb01(p3.b) * 100, 0, 100)
		};
	}

	function p3ToRgb(r: number, g: number, b: number) {
		const rp = decodeRgb01(clamp(r, 0, 100) / 100);
		const gp = decodeRgb01(clamp(g, 0, 100) / 100);
		const bp = decodeRgb01(clamp(b, 0, 100) / 100);

		const xyz = linearP3ToXyz(rp, gp, bp);

		return xyzToRgb(xyz.x, xyz.y, xyz.z);
	}

	function makeControls(model: ModelName): Control[] {
		switch (model) {
			case 'HSL': {
				const value = rgbToHsl(rgb);

				return [
					{
						key: 'h',
						label: 'Hue',
						min: 0,
						max: 360,
						step: 1,
						value: round(value.h, 1)
					},
					{
						key: 's',
						label: 'Saturation',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.s, 1)
					},
					{
						key: 'l',
						label: 'Lightness',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.l, 1)
					}
				];
			}

			case 'HSV': {
				const value = rgbToHsv(rgb);

				return [
					{
						key: 'h',
						label: 'Hue',
						min: 0,
						max: 360,
						step: 1,
						value: round(value.h, 1)
					},
					{
						key: 's',
						label: 'Saturation',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.s, 1)
					},
					{
						key: 'v',
						label: 'Value',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.v, 1)
					}
				];
			}

			case 'RGB':
				return [
					{
						key: 'r',
						label: 'Red',
						min: 0,
						max: 255,
						step: 1,
						value: Math.round(rgb.r)
					},
					{
						key: 'g',
						label: 'Green',
						min: 0,
						max: 255,
						step: 1,
						value: Math.round(rgb.g)
					},
					{
						key: 'b',
						label: 'Blue',
						min: 0,
						max: 255,
						step: 1,
						value: Math.round(rgb.b)
					}
				];

			case 'LCH': {
				const value = rgbToLch(rgb);

				return [
					{
						key: 'l',
						label: 'Lightness',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.l, 1)
					},
					{
						key: 'c',
						label: 'Chroma',
						min: 0,
						max: 150,
						step: 0.1,
						value: round(value.c, 1)
					},
					{
						key: 'h',
						label: 'Hue',
						min: 0,
						max: 360,
						step: 1,
						value: round(value.h, 1)
					}
				];
			}

			case 'OKLAB': {
				const value = rgbToOklab(rgb);

				return [
					{
						key: 'l',
						label: 'Lightness',
						min: 0,
						max: 1,
						step: 0.001,
						value: round(value.l, 3)
					},
					{
						key: 'a',
						label: 'A',
						min: -0.4,
						max: 0.4,
						step: 0.001,
						value: round(value.a, 3)
					},
					{
						key: 'b',
						label: 'B',
						min: -0.4,
						max: 0.4,
						step: 0.001,
						value: round(value.b, 3)
					}
				];
			}

			case 'OKLCH': {
				const value = rgbToOklch(rgb);

				return [
					{
						key: 'l',
						label: 'Lightness',
						min: 0,
						max: 1,
						step: 0.001,
						value: round(value.l, 3)
					},
					{
						key: 'c',
						label: 'Chroma',
						min: 0,
						max: 0.4,
						step: 0.001,
						value: round(value.c, 3)
					},
					{
						key: 'h',
						label: 'Hue',
						min: 0,
						max: 360,
						step: 1,
						value: round(value.h, 1)
					}
				];
			}

			case 'P3': {
				const value = rgbToP3(rgb);

				return [
					{
						key: 'r',
						label: 'Red',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.r, 1)
					},
					{
						key: 'g',
						label: 'Green',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.g, 1)
					},
					{
						key: 'b',
						label: 'Blue',
						min: 0,
						max: 100,
						step: 0.1,
						value: round(value.b, 1)
					}
				];
			}
			case 'HEX':
				return [];
		}
	}

	function syncControls() {
		controls = makeControls(activeModel);
		hexInput = rgbToHex(rgb);
	}

	function setModel(model: ModelName) {
		activeModel = model;
		syncControls();
	}

	function controlValue(key: string) {
		return controls.find((control) => control.key === key)?.value ?? 0;
	}

	// Samples what the resulting color would be for a given model
	// if one control's value were swapped out, holding the rest of
	// the current control values fixed. Used to paint each slider's
	// track with a gradient that actually reflects what it controls
	// (a hue slider shows the rainbow, a lightness slider shows
	// black-to-white, etc.) instead of a flat gray bar.
	function sampleModelColor(
		model: ModelName,
		values: Record<string, number>
	): RGB {
		switch (model) {
			case 'HSL':
				return hslToRgb(values.h, values.s, values.l);

			case 'HSV':
				return hsvToRgb(values.h, values.s, values.v);

			case 'RGB':
				return {
					r: values.r,
					g: values.g,
					b: values.b
				};

			case 'LCH':
				return lchToRgb(values.l, values.c, values.h);

			case 'OKLAB':
				return oklabToRgb(values.l, values.a, values.b);

			case 'OKLCH':
				return oklchToRgb(values.l, values.c, values.h);

			case 'P3':
				return p3ToRgb(values.r, values.g, values.b);

			case 'HEX':
				return rgb;
		}
	}

	// Every slider's gradient is anchored to fixed reference values
	// for every OTHER control, so no slider ever washes out toward
	// gray/black/white just because a sibling control's live value
	// happens to be at an extreme (e.g. Saturation going blank when
	// Lightness is 0 or 100). The one exception: for models with an
	// explicit Hue axis (HSL/HSV/LCH/OKLCH), the non-hue sliders
	// still pick up the LIVE hue -- so the identity of the color
	// still updates as you turn the hue dial, it just never lets S
	// or L/V/C dim each other out. RGB/P3/OKLAB have no hue concept,
	// so their sliders are fully fixed against neutral reference
	// values instead.
	const CONTROL_ANCHORS: Partial<
		Record<
			ModelName,
			Record<string, Record<string, number>>
		>
	> = {
		HSL: {
			h: { s: 100, l: 50 },
			s: { l: 50 },
			l: { s: 100 }
		},
		HSV: {
			h: { s: 100, v: 100 },
			s: { v: 100 },
			v: { s: 100 }
		},
		LCH: {
			h: { l: 65, c: 132 },
			l: { c: 132 },
			c: { l: 65 }
		},
		OKLCH: {
			h: { l: 0.75, c: 0.32 },
			l: { c: 0.32 },
			c: { l: 0.75 }
		},
		RGB: {
			r: { g: 128, b: 128 },
			g: { r: 128, b: 128 },
			b: { r: 128, g: 128 }
		},
		P3: {
			r: { g: 50, b: 50 },
			g: { r: 50, b: 50 },
			b: { r: 50, g: 50 }
		},
		OKLAB: {
			l: { a: 0, b: 0 },
			a: { l: 0.7, b: 0 },
			b: { l: 0.7, a: 0 }
		}
	};

	function controlGradient(control: Control): string {
		const base: Record<string, number> = {};

		for (const other of controls) {
			base[other.key] = other.value;
		}

		const anchor =
			CONTROL_ANCHORS[activeModel]?.[control.key] ?? {};

		const fixedValues = { ...base, ...anchor };

		const stopCount = 10;
		const stops: string[] = [];

		for (let i = 0; i <= stopCount; i++) {
			const t = i / stopCount;
			const value = control.min + (control.max - control.min) * t;

			const sampled = sampleModelColor(activeModel, {
				...fixedValues,
				[control.key]: value
			});

			stops.push(rgbToHex(sampled));
		}

		return `linear-gradient(to right, ${stops.join(', ')})`;
	}

	function updateRgbFromControls() {
		switch (activeModel) {
			case 'HSL':
				rgb = hslToRgb(
					controlValue('h'),
					controlValue('s'),
					controlValue('l')
				);
				break;

			case 'HSV':
				rgb = hsvToRgb(
					controlValue('h'),
					controlValue('s'),
					controlValue('v')
				);
				break;

			case 'RGB':
				rgb = {
					r: controlValue('r'),
					g: controlValue('g'),
					b: controlValue('b')
				};
				break;

			case 'LCH':
				rgb = lchToRgb(
					controlValue('l'),
					controlValue('c'),
					controlValue('h')
				);
				break;

			case 'OKLAB':
				rgb = oklabToRgb(
					controlValue('l'),
					controlValue('a'),
					controlValue('b')
				);
				break;

			case 'OKLCH':
				rgb = oklchToRgb(
					controlValue('l'),
					controlValue('c'),
					controlValue('h')
				);
				break;

			case 'P3':
				rgb = p3ToRgb(
					controlValue('r'),
					controlValue('g'),
					controlValue('b')
				);
				break;
		}

		hexInput = rgbToHex(rgb);
	}

	function updateControl(index: number, value: number) {
		const control = controls[index];

		control.value = clamp(
			value,
			control.min,
			control.max
		);

		controls = [...controls];

		updateRgbFromControls();
	}

	// Shift = jump by 10, Ctrl/Cmd = fine adjustment (a tenth of the
	// normal step), plain arrow = the control's normal step.
	function handleSliderKeydown(
		event: KeyboardEvent,
		index: number,
		control: Control
	) {
		const isIncrement =
			event.key === 'ArrowUp' || event.key === 'ArrowRight';
		const isDecrement =
			event.key === 'ArrowDown' || event.key === 'ArrowLeft';

		if (!isIncrement && !isDecrement) {
			return;
		}

		event.preventDefault();

		let delta = control.step;

		if (event.shiftKey) {
			delta = 10;
		} else if (event.ctrlKey || event.metaKey) {
			delta = Math.max(control.step / 10, 0.001);
		}

		updateControl(
			index,
			control.value + (isIncrement ? delta : -delta)
		);
	}

	// Dragging the slider normally uses the browser's own
	// click-position behavior. Holding Shift or Ctrl/Cmd while
	// dragging instead moves the value relative to how far the
	// pointer has traveled, for coarse (+/-10 per notch) or fine
	// adjustment -- since a native <input type="range"> has no
	// built-in way to change drag sensitivity based on a modifier
	// key, this takes over the drag manually only while a modifier
	// is held at the moment the drag starts.
	let sliderDrag: {
		pointerId: number;
		index: number;
		control: Control;
		startX: number;
		startValue: number;
		mode: 'coarse' | 'fine';
	} | null = null;

	function handleSliderPointerDown(
		event: PointerEvent,
		index: number,
		control: Control
	) {
		if (!event.shiftKey && !event.ctrlKey && !event.metaKey) {
			return;
		}

		event.preventDefault();

		sliderDrag = {
			pointerId: event.pointerId,
			index,
			control,
			startX: event.clientX,
			startValue: control.value,
			mode: event.shiftKey ? 'coarse' : 'fine'
		};

		window.addEventListener(
			'pointermove',
			handleSliderPointerMove
		);
		window.addEventListener(
			'pointerup',
			handleSliderPointerUp
		);
	}

	function handleSliderPointerMove(event: PointerEvent) {
		if (
			!sliderDrag ||
			event.pointerId !== sliderDrag.pointerId
		) {
			return;
		}

		const deltaX = event.clientX - sliderDrag.startX;
		const range =
			sliderDrag.control.max - sliderDrag.control.min;

		// Coarse: roughly 10 units per 20px dragged.
		// Fine: the full track width covers a small fraction of
		// the range, for precise adjustment.
		const unitsPerPixel =
			sliderDrag.mode === 'coarse'
				? 10 / 20
				: range / 2000;

		updateControl(
			sliderDrag.index,
			sliderDrag.startValue + deltaX * unitsPerPixel
		);
	}

	function handleSliderPointerUp(event: PointerEvent) {
		if (
			!sliderDrag ||
			event.pointerId !== sliderDrag.pointerId
		) {
			return;
		}

		sliderDrag = null;

		window.removeEventListener(
			'pointermove',
			handleSliderPointerMove
		);
		window.removeEventListener(
			'pointerup',
			handleSliderPointerUp
		);
	}

	function handleHexInput(value: string) {
		hexInput = value.toUpperCase();

		const parsed = hexToRgb(hexInput);

		if (parsed) {
			rgb = parsed;
		}
	}

	function commitHex() {
		const parsed = hexToRgb(hexInput);

		if (parsed) {
			rgb = parsed;
			hexInput = rgbToHex(parsed);
		} else {
			hexInput = rgbToHex(rgb);
		}
	}

	function scrubValue(
		event: PointerEvent,
		index: number
	) {
		if (event.button !== 0) return;

		const control = controls[index];

		const startX = event.clientX;
		const startValue = control.value;

		const span = control.max - control.min;

		const pixelsForRange = 180;

		const onMove = (moveEvent: PointerEvent) => {
			const deltaX = moveEvent.clientX - startX;

			const next =
				startValue +
				(deltaX / pixelsForRange) * span;

			const stepped =
				Math.round(next / control.step) *
				control.step;

			updateControl(index, stepped);
		};

		const onUp = () => {
			window.removeEventListener('pointermove', onMove);
			window.removeEventListener('pointerup', onUp);
		};

		window.addEventListener('pointermove', onMove);
		window.addEventListener('pointerup', onUp);
	}

	async function closeWindow() {
		await getCurrentWindow().close();
	}

	async function startDragging(event: MouseEvent) {
		const target = event.target as HTMLElement;

		if (
			target.closest('button') ||
			target.closest('input')
		) {
			return;
		}

		event.preventDefault();

		await getCurrentWindow().startDragging();
	}

	async function startResize(
		event: MouseEvent,
		direction:
			| 'NorthWest'
			| 'NorthEast'
			| 'SouthWest'
			| 'SouthEast'
	) {
		event.preventDefault();

		await getCurrentWindow().startResizeDragging(direction);
	}

	function updateScale() {
		uiScale = Math.min(
			window.innerWidth / REFERENCE_WIDTH,
			window.innerHeight / REFERENCE_HEIGHT
		);

		document.documentElement.style.setProperty(
			'--ui-scale',
			String(uiScale)
		);
	}

	async function enforceAspectRatio(attempt = 0) {
		const appWindow = getCurrentWindow();

		// innerSize() returns PHYSICAL pixels, but LogicalSize()
		// expects LOGICAL pixels. On any display with a DPI scale
		// factor other than 1, mixing the two causes the window to
		// grow/shrink incorrectly on every resize event.
		const scaleFactor = await appWindow.scaleFactor();
		const physicalSize = await appWindow.innerSize();
		const logicalSize = physicalSize.toLogical(scaleFactor);

		const width = logicalSize.width;
		const height = logicalSize.height;

		const expectedHeight = width / REFERENCE_RATIO;

		if (Math.abs(height - expectedHeight) < 2) {
			return;
		}

		await appWindow.setSize(
			new LogicalSize(
				width,
				expectedHeight
			)
		);

		// Some window managers (tiling WMs, or ones that clamp to
		// minHeight/minHeight constraints, or just apply the resize
		// asynchronously) don't honor setSize() on the first try, or
		// apply it a frame late. Re-check shortly after and correct
		// again if it didn't stick, instead of silently leaving
		// dead space. Capped so a WM that refuses entirely can't
		// loop forever.
		if (attempt < 3) {
			setTimeout(() => {
				enforceAspectRatio(attempt + 1);
			}, 60);
		}
	}

	// onResized fires rapidly (many times per second) during a
	// live OS resize drag. enforceAspectRatio() makes several
	// awaited IPC round-trips to the Rust backend, so calling it
	// on every single event causes overlapping, racing corrections
	// that fight each other and the live drag -- this is what was
	// producing stuck dead space and rendering glitches. Instead,
	// only run the actual correction once resize activity settles.
	function scheduleAspectRatioCorrection() {
		if (resizeSettleTimer) {
			clearTimeout(resizeSettleTimer);
		}

		resizeSettleTimer = setTimeout(() => {
			resizeSettleTimer = null;
			enforceAspectRatio();
		}, 120);
	}

	function showToast(message: string) {
		toastText = message;
		toastVisible = true;

		if (toastTimer) {
			clearTimeout(toastTimer);
		}

		toastTimer = setTimeout(() => {
			toastVisible = false;
		}, 1800);
	}

	function saveHistory() {
		localStorage.setItem(
			'procolorator-history',
			JSON.stringify(history)
		);
	}

	function addToHistory(hex: string) {
		const parsed = hexToRgb(hex) ?? rgb;

		history = [
			{
				hex,
				name: nearestColorName(parsed),
				createdAt: Date.now()
			},
			...history.filter(
				(item) => item.hex !== hex
			)
		].slice(0, 30);

		saveHistory();
	}

	async function copyColor(hex = rgbToHex(rgb)) {
		try {
			await navigator.clipboard.writeText(hex);
		} catch {
			const textarea =
				document.createElement('textarea');

			textarea.value = hex;
			document.body.appendChild(textarea);

			textarea.select();

			document.execCommand('copy');

			textarea.remove();
		}

		addToHistory(hex);

		showToast(`${hex} is copied to clipboard`);
	}

	function setHistoryView(
		view: 'list' | 'grid'
	) {
		historyView = view;

		localStorage.setItem(
			'procolorator-history-view',
			view
		);
	}

	onMount(() => {
		getCurrentWindow()
			.setFocus()
			.catch(() => {});

		const savedHistory =
			localStorage.getItem(
				'procolorator-history'
			);

		if (savedHistory) {
			try {
				history = JSON.parse(savedHistory);
			} catch {
				history = [];
			}
		}

		const savedView =
			localStorage.getItem(
				'procolorator-history-view'
			);

		if (
			savedView === 'list' ||
			savedView === 'grid'
		) {
			historyView = savedView;
		}

		syncControls();
		updateScale();

		const appWindow = getCurrentWindow();

		enforceAspectRatio();

		const unlistenResized = appWindow.onResized(
			() => {
				updateScale();
				scheduleAspectRatioCorrection();
			}
		);

		const unlistenColorPicked = listen<string>(
			'color-picked',
			(event) => {
				const parsed = hexToRgb(event.payload);

				if (parsed) {
					rgb = parsed;
					syncControls();
				}

				addToHistory(event.payload);
				activeTab = 'selector';
				showToast(`${event.payload} is copied to clipboard`);
			}
		);

		window.addEventListener(
			'resize',
			updateScale
		);

		return () => {
			window.removeEventListener(
				'resize',
				updateScale
			);

			if (resizeSettleTimer) {
				clearTimeout(resizeSettleTimer);
			}

			unlistenResized.then((unlisten) =>
				unlisten()
			);

			unlistenColorPicked.then((unlisten) =>
				unlisten()
			);
		};
	});

	// The eyedropper itself already copies to the clipboard the
	// instant you click a pixel (see the picker route) -- this just
	// launches it. Also reachable via the global shortcut registered
	// in Rust, so this button mainly exists for discoverability and
	// for testing on systems where the shortcut might not register
	// (already-taken combo, permissions, etc.).
	async function openEyedropper() {
		try {
			await invoke('open_color_picker');
		} catch (error) {
			console.error('Failed to open eyedropper', error);
		}
	}
</script>

<div class="viewport">
	<div
		class="design-stage"
		style={`transform: scale(${uiScale})`}
	>
		<div class="app-shell">
			<header
				class="titlebar"
				data-tauri-drag-region
				on:mousedown={startDragging}
			>
				<div
					class="app-title"
					data-tauri-drag-region
				>
					ProColorator
				</div>

				<button
					class="close-button"
					type="button"
					aria-label="Close ProColorator"
					on:click={closeWindow}
				>
					<svg
						width="20"
						height="20"
						viewBox="0 0 20 20"
						fill="none"
						aria-hidden="true"
					>
						<path
							d="M5 5L15 15M15 5L5 15"
							stroke="currentColor"
							stroke-width="1.6"
							stroke-linecap="round"
						/>
					</svg>
				</button>
			</header>

			<nav class="tabs">
				<button
					class:active={activeTab === 'selector'}
					class="tab-button"
					type="button"
					on:click={() =>
						(activeTab = 'selector')}
				>
					<svg
						class="tab-icon"
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						aria-hidden="true"
					>
						<path
							d="m2 22 1-1h3l9-9"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
						<path
							d="M3 21v-3l9-9"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
						<path
							d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
					</svg>

					<span>Selector</span>
				</button>

				<button
					class:active={activeTab === 'history'}
					class="tab-button"
					type="button"
					on:click={() =>
						(activeTab = 'history')}
				>
					<svg
						class="tab-icon"
						width="20"
						height="20"
						viewBox="0 0 20 20"
						fill="none"
						aria-hidden="true"
					>
						<path
							d="M4.8 5.5H15.2M4.8 10H15.2M4.8 14.5H15.2"
							stroke="currentColor"
							stroke-width="1.5"
							stroke-linecap="round"
						/>
						<circle
							cx="2.7"
							cy="5.5"
							r="0.9"
							fill="currentColor"
						/>
						<circle
							cx="2.7"
							cy="10"
							r="0.9"
							fill="currentColor"
						/>
						<circle
							cx="2.7"
							cy="14.5"
							r="0.9"
							fill="currentColor"
						/>
					</svg>

					<span>History</span>
				</button>
			</nav>

			{#if activeTab === 'selector'}
				<main class="selector-page">
					<section
						class="color-preview"
						style={`background: ${rgbToHex(rgb)}`}
					>
						<button
							type="button"
							class="eyedropper-button"
							on:click={openEyedropper}
							title="Pick a color from your screen (Ctrl+Shift+C)"
							aria-label="Pick a color from your screen"
						>
							<svg
								width="18"
								height="18"
								viewBox="0 0 24 24"
								fill="none"
								aria-hidden="true"
							>
								<path
									d="m2 22 1-1h3l9-9"
									stroke="currentColor"
									stroke-width="1.8"
									stroke-linecap="round"
									stroke-linejoin="round"
								/>
								<path
									d="M3 21v-3l9-9"
									stroke="currentColor"
									stroke-width="1.8"
									stroke-linecap="round"
									stroke-linejoin="round"
								/>
								<path
									d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"
									stroke="currentColor"
									stroke-width="1.8"
									stroke-linecap="round"
									stroke-linejoin="round"
								/>
							</svg>
						</button>
					</section>

					<section class="model-grid">
						{#each models as model}
							<button
								type="button"
								class="model-button"
								class:active={activeModel === model}
								on:click={() => setModel(model)}
							>
								{model}
							</button>
						{/each}
					</section>

					<section class="controls-panel">
						{#if activeModel === 'HEX'}
							<div class="hex-control">
								<label for="hex-value">
									HEX
								</label>

								<input
									id="hex-value"
									class="hex-input"
									type="text"
									maxlength="7"
									spellcheck="false"
									value={hexInput}
									on:input={(event) =>
										handleHexInput(
											(
												event.currentTarget as HTMLInputElement
											).value
										)}
									on:blur={commitHex}
									on:keydown={(event) => {
										if (
											event.key ===
											'Enter'
										) {
											(
												event.currentTarget as HTMLInputElement
											).blur();
										}
									}}
								/>
							</div>
						{:else}
							<p class="control-hint">
								Shift + drag or arrow keys for ±10 · Ctrl/Cmd for fine adjustment
							</p>

							{#each controls as control, index}
								<div class="control-row">
									<label class="control-label">
										{control.label}
									</label>

									<div class="control-inputs">
										<div class="slider-wrapper">
											<div
												class="slider-track-fill"
												style={`background: ${controlGradient(control)}`}
											></div>

											<input
												class="range-slider"
												type="range"
												min={control.min}
												max={control.max}
												step={control.step}
												value={control.value}
												on:input={(event) =>
													updateControl(
														index,
														Number(
															(
																event.currentTarget as HTMLInputElement
															).value
														)
													)}
												on:keydown={(event) =>
													handleSliderKeydown(
														event,
														index,
														control
													)}
												on:pointerdown={(event) =>
													handleSliderPointerDown(
														event,
														index,
														control
													)}
											/>
										</div>

										<input
											class="number-input"
											type="number"
											inputmode="decimal"
											min={control.min}
											max={control.max}
											step={control.step}
											value={control.value}
											on:input={(event) =>
												updateControl(
													index,
													Number(
														(
															event.currentTarget as HTMLInputElement
														).value
													)
												)}
											on:pointerdown={(event) =>
												scrubValue(
													event,
													index
												)}
											aria-label={`${control.label} value`}
										/>
									</div>
								</div>
							{/each}
						{/if}

						<button
							type="button"
							class="copy-button"
							on:click={() => copyColor()}
						>
							<svg
								class="copy-icon"
								xmlns="http://www.w3.org/2000/svg"
								width="20"
								height="20"
								fill="currentColor"
								viewBox="0 0 256 256"
								aria-hidden="true"
							>
								<path
									d="M216,32H88a8,8,0,0,0-8,8V80H40a8,8,0,0,0-8,8V216a8,8,0,0,0,8,8H168a8,8,0,0,0,8-8V176h40a8,8,0,0,0,8-8V40A8,8,0,0,0,216,32ZM160,208H48V96H160Zm48-48H176V88a8,8,0,0,0-8-8H96V48H208Z"
								/>
							</svg>

							<span>Copy color</span>
						</button>
					</section>
				</main>
			{:else}
				<main class="history-page">
					{#if history.length > 0}
						<div class="history-header">
							<p>
								Click on color to copy to clipboard
							</p>

							<div class="history-view-toggle">
								<button
									type="button"
									class:active={historyView === 'list'}
									on:click={() =>
										setHistoryView('list')}
									aria-label="List view"
								>
									<svg
										width="20"
										height="20"
										viewBox="0 0 20 20"
										fill="none"
									>
										<path
											d="M6 5H16M6 10H16M6 15H16"
											stroke="currentColor"
											stroke-width="1.5"
											stroke-linecap="round"
										/>
										<circle
											cx="3"
											cy="5"
											r="1"
											fill="currentColor"
										/>
										<circle
											cx="3"
											cy="10"
											r="1"
											fill="currentColor"
										/>
										<circle
											cx="3"
											cy="15"
											r="1"
											fill="currentColor"
										/>
									</svg>
								</button>

								<button
									type="button"
									class:active={historyView === 'grid'}
									on:click={() =>
										setHistoryView('grid')}
									aria-label="Grid view"
								>
									<svg
										width="20"
										height="20"
										viewBox="0 0 20 20"
										fill="none"
									>
										<rect
											x="3"
											y="3"
											width="5"
											height="5"
											r="0.5"
											stroke="currentColor"
											stroke-width="1.3"
										/>
										<rect
											x="12"
											y="3"
											width="5"
											height="5"
											r="0.5"
											stroke="currentColor"
											stroke-width="1.3"
										/>
										<rect
											x="3"
											y="12"
											width="5"
											height="5"
											r="0.5"
											stroke="currentColor"
											stroke-width="1.3"
										/>
										<rect
											x="12"
											y="12"
											width="5"
											height="5"
											r="0.5"
											stroke="currentColor"
											stroke-width="1.3"
										/>
									</svg>
								</button>
							</div>
						</div>
					{/if}

					<div class="history-scroll">
						{#if history.length === 0}
							<div class="history-empty">
								<h2>History is empty</h2>
								<p>
									Selected colors through selector
									are shown here
								</p>
							</div>
						{:else if historyView === 'list'}
							<div class="history-list">
								{#each history as color}
									<button
										type="button"
										class="history-row"
										on:click={() =>
											copyColor(color.hex)}
									>
										<div class="history-text">
											<span class="history-name">
												{color.name}
											</span>

											<span class="history-hex">
												{color.hex}
											</span>
										</div>

										<div
											class="history-swatch"
											style={`background:${color.hex}`}
										></div>
									</button>
								{/each}
							</div>
						{:else}
							<div class="history-grid">
								{#each history as color}
									<button
										type="button"
										class="history-card"
										on:click={() =>
											copyColor(color.hex)}
									>
										<div
											class="history-card-color"
											style={`background:${color.hex}`}
										></div>

										<span class="history-card-hex">
											{color.hex}
										</span>

										<span class="history-card-name">
											{color.name}
										</span>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				</main>
			{/if}

			{#if toastVisible}
				<div
					class="copy-toast"
					role="status"
					aria-live="polite"
				>
					<svg
						class="toast-icon"
						width="20"
						height="20"
						viewBox="0 0 256 256"
						fill="none"
						aria-hidden="true"
					>
						<circle
							cx="128"
							cy="128"
							r="96"
							stroke="currentColor"
							stroke-width="16"
						/>
						<path
							d="M128 112V176"
							stroke="currentColor"
							stroke-width="16"
							stroke-linecap="round"
						/>
						<circle
							cx="128"
							cy="80"
							r="9"
							fill="currentColor"
						/>
					</svg>

					<span class="toast-text">{toastText}</span>
				</div>
			{/if}

			<div
				class="resize-corner resize-nw"
				on:mousedown={(event) =>
					startResize(event, 'NorthWest')}
				aria-hidden="true"
			></div>

			<div
				class="resize-corner resize-ne"
				on:mousedown={(event) =>
					startResize(event, 'NorthEast')}
				aria-hidden="true"
			></div>

			<div
				class="resize-corner resize-sw"
				on:mousedown={(event) =>
					startResize(event, 'SouthWest')}
				aria-hidden="true"
			></div>

			<div
				class="resize-corner resize-se"
				on:mousedown={(event) =>
					startResize(event, 'SouthEast')}
				aria-hidden="true"
			></div>
		</div>
	</div>
</div>
