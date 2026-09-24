<script lang="ts">
	import { onMount } from 'svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { invoke } from '@tauri-apps/api/core';

	// This route only runs inside the dedicated "picker" window
	// created by the Rust `open_color_picker` command. That window
	// is a normal OPAQUE window (no transparency -- see the note in
	// lib.rs about why) sized to exactly cover the primary monitor.
	// A single screenshot was captured the instant the window was
	// requested; this page fetches that one image, displays it as
	// its own background, and does all magnifier zooming and pixel
	// sampling by reading directly out of that in-memory bitmap.
	// No further IPC calls happen after the initial fetch.

	const MIN_RADIUS = 8;
	const MAX_RADIUS = 100;
	const DISPLAY_SIZE = 160; // CSS px the magnifier canvas renders at

	let radius = 30;
	let localX = 0;
	let localY = 0;
	let lastColor = { r: 255, g: 255, b: 255 };
	let hexText = '#FFFFFF';
	let ready = false;
	let loadError = '';

	let backgroundCanvasEl: HTMLCanvasElement; // full-size, hidden, holds the captured screenshot
	let visibleCanvasEl: HTMLCanvasElement; // scaled to fill the window, what you actually see
	let magnifierCanvasEl: HTMLCanvasElement;

	let backgroundCtx: CanvasRenderingContext2D | null = null;
	let bgWidth = 0;
	let bgHeight = 0;
	let scaleFactor = 1;

	const clamp = (value: number, min: number, max: number) =>
		Math.min(max, Math.max(min, value));

	function componentToHex(value: number) {
		return clamp(Math.round(value), 0, 255)
			.toString(16)
			.padStart(2, '0')
			.toUpperCase();
	}

	function toHex(color: { r: number; g: number; b: number }) {
		return `#${componentToHex(color.r)}${componentToHex(color.g)}${componentToHex(color.b)}`;
	}

	function toPhysical(localValue: number) {
		return Math.round(localValue * scaleFactor);
	}

	function sampleAndDraw(physicalX: number, physicalY: number) {
		if (!backgroundCtx) return;

		const clampedX = clamp(physicalX, 0, bgWidth - 1);
		const clampedY = clamp(physicalY, 0, bgHeight - 1);

		// Center pixel color, straight from the already-decoded image.
		const centerData = backgroundCtx.getImageData(
			clampedX,
			clampedY,
			1,
			1
		).data;

		lastColor = {
			r: centerData[0],
			g: centerData[1],
			b: centerData[2]
		};
		hexText = toHex(lastColor);

		// Zoomed crop for the magnifier panel.
		const size = radius * 2 + 1;
		const srcX = clamp(clampedX - radius, 0, Math.max(0, bgWidth - size));
		const srcY = clamp(clampedY - radius, 0, Math.max(0, bgHeight - size));
		const srcW = Math.min(size, bgWidth);
		const srcH = Math.min(size, bgHeight);

		const magCtx = magnifierCanvasEl?.getContext('2d');
		if (magCtx && backgroundCanvasEl) {
			magCtx.imageSmoothingEnabled = false;
			magCtx.clearRect(0, 0, DISPLAY_SIZE, DISPLAY_SIZE);
			magCtx.drawImage(
				backgroundCanvasEl,
				srcX,
				srcY,
				srcW,
				srcH,
				0,
				0,
				DISPLAY_SIZE,
				DISPLAY_SIZE
			);
		}
	}

	function handlePointerMove(event: PointerEvent) {
		localX = event.clientX;
		localY = event.clientY;

		sampleAndDraw(toPhysical(localX), toPhysical(localY));
	}

	function handleWheel(event: WheelEvent) {
		event.preventDefault();

		const step = Math.max(2, Math.round(radius * 0.15));
		radius = clamp(
			radius + (event.deltaY > 0 ? step : -step),
			MIN_RADIUS,
			MAX_RADIUS
		);

		sampleAndDraw(toPhysical(localX), toPhysical(localY));
	}

	async function copyToClipboard(hex: string) {
		try {
			await navigator.clipboard.writeText(hex);
		} catch {
			const textarea = document.createElement('textarea');
			textarea.value = hex;
			document.body.appendChild(textarea);
			textarea.select();
			document.execCommand('copy');
			textarea.remove();
		}
	}

	async function handleConfirm(event: MouseEvent) {
		event.preventDefault();

		if (!ready) return;

		// Clipboard copy is a nice-to-have; it must never block or
		// break the handoff back to the main window.
		copyToClipboard(hexText).catch((error) => {
			console.error('Clipboard copy failed', error);
		});

		try {
			await invoke('finish_picking', { hex: hexText });
		} catch (error) {
			console.error(
				'finish_picking failed, closing picker directly',
				error
			);

			// Fallback so the overlay can never get stuck open even
			// if the Rust-side handoff throws for some reason.
			try {
				await getCurrentWindow().close();
			} catch (closeError) {
				console.error(
					'Fallback close also failed',
					closeError
				);
			}
		}
	}

	async function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			try {
				await invoke('cancel_picking');
			} catch (error) {
				console.error('Failed to cancel picking', error);
			}
		}
	}

	$: panelLeft = clamp(
		localX + 24,
		0,
		(typeof window !== 'undefined' ? window.innerWidth : 0) -
			DISPLAY_SIZE -
			16
	);
	$: panelTop = clamp(
		localY + 24,
		0,
		(typeof window !== 'undefined' ? window.innerHeight : 0) -
			DISPLAY_SIZE -
			70
	);

	onMount(() => {
		const appWindow = getCurrentWindow();

		appWindow.setFocus().catch(() => {});

		(async () => {
			try {
				scaleFactor = await appWindow.scaleFactor();

				const bytes = await invoke<number[]>(
					'get_picker_background'
				);

				const blob = new Blob([new Uint8Array(bytes)], {
					type: 'image/png'
				});
				const objectUrl = URL.createObjectURL(blob);

				const img = new Image();

				await new Promise<void>((resolve, reject) => {
					img.onload = () => resolve();
					img.onerror = () =>
						reject(new Error('Failed to decode captured screenshot'));
					img.src = objectUrl;
				});

				bgWidth = img.naturalWidth;
				bgHeight = img.naturalHeight;

				backgroundCanvasEl.width = bgWidth;
				backgroundCanvasEl.height = bgHeight;
				backgroundCtx = backgroundCanvasEl.getContext('2d', {
					willReadFrequently: true
				});
				backgroundCtx?.drawImage(img, 0, 0);

				visibleCanvasEl.width = bgWidth;
				visibleCanvasEl.height = bgHeight;
				const visibleCtx = visibleCanvasEl.getContext('2d');
				visibleCtx?.drawImage(img, 0, 0);

				URL.revokeObjectURL(objectUrl);

				ready = true;
			} catch (error) {
				console.error('Picker failed to load background', error);
				loadError =
					'Could not load the screen capture. Press Escape to close.';
			}
		})();

		window.addEventListener('keydown', handleKeydown);

		return () => {
			window.removeEventListener('keydown', handleKeydown);
		};
	});
</script>

<svelte:window on:pointermove={handlePointerMove} on:wheel={handleWheel} />

<div class="picker-root" role="button" tabindex="0" on:click={handleConfirm}>
	<canvas bind:this={backgroundCanvasEl} class="offscreen-canvas"></canvas>
	<canvas bind:this={visibleCanvasEl} class="visible-canvas"></canvas>

	{#if loadError}
		<div class="load-error">{loadError}</div>
	{/if}

	{#if ready}
		<div
			class="magnifier"
			style={`left:${panelLeft}px; top:${panelTop}px;`}
		>
			<div class="magnifier-canvas-wrap">
				<canvas bind:this={magnifierCanvasEl} width={DISPLAY_SIZE} height={DISPLAY_SIZE} class="magnifier-canvas"
				></canvas>
				<div class="magnifier-crosshair"></div>
			</div>

			<div class="magnifier-readout">
				<span class="magnifier-swatch" style={`background:${hexText}`}></span>
				<span class="magnifier-hex">{hexText}</span>
			</div>

			<div class="magnifier-hint">
				Click to copy &middot; Scroll to zoom &middot; Esc to cancel
			</div>
		</div>
	{/if}
</div>

<style>
	.picker-root {
		position: fixed;
		inset: 0;

		width: 100vw;
		height: 100vh;

		overflow: hidden;

		cursor: crosshair;
	}

	.offscreen-canvas {
		display: none;
	}

	.visible-canvas {
		position: absolute;
		top: 0;
		left: 0;

		width: 100%;
		height: 100%;

		pointer-events: none;
	}

	.load-error {
		position: fixed;
		top: 50%;
		left: 50%;

		transform: translate(-50%, -50%);

		padding: 16px 20px;

		border-radius: 8px;

		background: rgba(10, 8, 0, 0.9);
		color: #ffffff;

		font-family: sans-serif;
		font-size: 14px;
	}

	.magnifier {
		position: fixed;

		display: flex;
		flex-direction: column;

		gap: 8px;

		padding: 10px;

		border-radius: 10px;

		background: rgba(10, 8, 0, 0.85);

		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);

		pointer-events: none;

		z-index: 10;
	}

	.magnifier-canvas-wrap {
		position: relative;

		width: 160px;
		height: 160px;

		overflow: hidden;

		border-radius: 6px;

		border: 1px solid rgba(255, 255, 255, 0.25);
	}

	.magnifier-canvas {
		width: 100%;
		height: 100%;

		image-rendering: pixelated;
	}

	.magnifier-crosshair {
		position: absolute;

		top: 50%;
		left: 50%;

		width: 12px;
		height: 12px;

		transform: translate(-50%, -50%);

		border: 1.5px solid rgba(255, 255, 255, 0.9);
		border-radius: 50%;

		box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.6);
	}

	.magnifier-readout {
		display: flex;
		align-items: center;

		gap: 8px;
	}

	.magnifier-swatch {
		width: 20px;
		height: 20px;

		flex: 0 0 20px;

		border-radius: 4px;

		border: 1px solid rgba(255, 255, 255, 0.3);
	}

	.magnifier-hex {
		color: #ffffff;

		font-size: 14px;
		font-weight: 500;

		font-family:
			'Inclusive Sans',
			sans-serif;
	}

	.magnifier-hint {
		color: rgba(255, 255, 255, 0.6);

		font-size: 11px;

		white-space: nowrap;
	}
</style>
