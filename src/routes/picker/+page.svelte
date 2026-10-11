<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	type Frame = { rgba: number[]; width: number; height: number; hex: string; sequence: number };
	let canvas: HTMLCanvasElement;
	let hex = '#FFFFFF';
	let error = '';
	onMount(() => {
		let stopped = false;
		let timer: ReturnType<typeof setTimeout>;
		let sequence = -1;
		const source = document.createElement('canvas');
		async function refresh() {
			try {
				const frame = await invoke<Frame | null>('get_picker_frame');
				if (stopped) return;
				if (frame && frame.sequence !== sequence) {
					sequence = frame.sequence;
					hex = frame.hex;
					source.width = frame.width;
					source.height = frame.height;
					source.getContext('2d')!.putImageData(new ImageData(
						new Uint8ClampedArray(frame.rgba), frame.width, frame.height), 0, 0);
					const context = canvas.getContext('2d')!;
					context.imageSmoothingEnabled = false;
					context.clearRect(0, 0, 160, 160);
					context.drawImage(source, 0, 0, 160, 160);
				}
			} catch (cause) {
				if (stopped) return;
				error = String(cause);
				await invoke('cancel_picking').catch(() => {});
				return;
			}
			if (!stopped) timer = setTimeout(refresh, 50);
		}
		invoke('picker_ready').then(refresh).catch((cause) => {
			error = String(cause);
			invoke('cancel_picking').catch(() => {});
		});
		return () => { stopped = true; clearTimeout(timer); };
	});
</script>

<div class="magnifier">
	<div class="magnifier-canvas-wrap">
		<canvas bind:this={canvas} width="160" height="160"></canvas>
		<div class="crosshair"></div>
	</div>
	<div class="readout"><span class="swatch" style={`background:${hex}`}></span>{hex}</div>
	<div class="hint">Click or Enter to copy<br />Esc to cancel · + / − to zoom</div>
	{#if error}<div role="alert">{error}</div>{/if}
</div>

<style>
	.magnifier { height: 100vh; display: flex; flex-direction: column; align-items: center;
		gap: 10px; padding: 12px; background: #150f00; color: white; }
	.magnifier-canvas-wrap { position: relative; width: 160px; height: 160px;
		overflow: hidden; border-radius: 6px; border: 1px solid #ffffff40; }
	canvas { width: 100%; height: 100%; image-rendering: pixelated; }
	.crosshair { position: absolute; top: 50%; left: 50%; width: 12px; height: 12px;
		transform: translate(-50%, -50%); border: 1.5px solid white; border-radius: 50%;
		box-shadow: 0 0 0 1px #000; }
	.readout { display: flex; align-items: center; gap: 8px; font-size: 14px; }
	.swatch { width: 20px; height: 20px; border-radius: 4px; border: 1px solid #ffffff50; }
	.hint { color: #ffffff99; font-size: 11px; text-align: center; line-height: 1.5; }
</style>
