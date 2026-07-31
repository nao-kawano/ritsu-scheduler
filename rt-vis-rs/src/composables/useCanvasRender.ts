// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================

export interface ThemeStyles {
  text: string;
  textDim: string;
  border: string;
  surfaceHeader: string;
  surface: string;
  gridMajor: string;
  gridMinor: string;
  fontSizePx: string;
  fontFamily: string;
}

export interface RenderHeaderOptions {
  scrollLeft: number;
  width: number;
  height: number;
  totalCycles: number;
  cycleTimeMs: number;
  majorPx: number;
  styles: ThemeStyles;
}

export interface RenderGridOptions {
  scrollLeft: number;
  width: number;
  height: number;
  totalCycles: number;
  majorPx: number;
  minorPx: number;
  styles: ThemeStyles;
}

// Layout constants for header rendering (matching DOM-based Create Mode pixel alignment)
const HEADER_PADDING_LEFT = 8;
const HEADER_LABEL_CYCLE_Y = 6;
const HEADER_LABEL_TIME_Y = 21;
const HEADER_TEXT_DIM_ALPHA = 0.8;

/**
 * Shared Canvas Rendering Utilities Composable.
 * Provides CSS theme token extraction, high-DPI canvas buffer resizing,
 * and unified sticky timeline header rendering.
 */
export function useCanvasRender() {
  /**
   * Safely extract computed CSS design tokens and fonts from a DOM element.
   * Fallback values adhere to design system theme defaults.
   */
  const getThemeStyles = (el: HTMLElement): ThemeStyles => {
    const style = getComputedStyle(el);
    const fontSizePx = style.getPropertyValue('--rt-font-xs').trim() || '11px';
    const computedFontFamily = style.fontFamily ? style.fontFamily.trim() : '';

    return {
      text: style.getPropertyValue('--rt-color-text').trim() || 'rgba(0, 0, 0, 0.87)',
      textDim: style.getPropertyValue('--rt-color-text-dim').trim() || 'rgba(0, 0, 0, 0.6)',
      border: style.getPropertyValue('--rt-color-border').trim() || 'rgba(0, 0, 0, 0.12)',
      surfaceHeader: style.getPropertyValue('--rt-color-surface-header').trim() || '#f5f5f5',
      surface: style.getPropertyValue('--rt-color-surface').trim() || '#ffffff',
      gridMajor: style.getPropertyValue('--rt-grid-major').trim() || 'rgba(0, 0, 0, 0.3)',
      gridMinor: style.getPropertyValue('--rt-grid-minor').trim() || 'rgba(0, 0, 0, 0.1)',
      fontSizePx,
      fontFamily: computedFontFamily || style.getPropertyValue('font-family').trim() || 'Inter, system-ui, sans-serif'
    };
  };

  /**
   * Adjust canvas physical buffer resolution and CSS display size according to devicePixelRatio.
   * Returns scaled 2D rendering context or null if unrenderable.
   */
  const prepareCanvas = (
    canvas: HTMLCanvasElement,
    container: HTMLElement
  ): { width: number; height: number; dpr: number; ctx: CanvasRenderingContext2D | null } => {
    const dpr = window.devicePixelRatio || 1;
    // Use clientWidth / clientHeight to get exact inner viewport bounds (excluding scrollbar gutters)
    const width = container.clientWidth || container.getBoundingClientRect().width;
    const height = container.clientHeight || container.getBoundingClientRect().height;

    if (width === 0 || height === 0) {
      return { width: 0, height: 0, dpr, ctx: null };
    }

    // Synchronize CSS display bounds with viewport container
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    // Scale physical resolution buffer
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);

    const ctx = canvas.getContext('2d');
    if (ctx) {
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, width, height);
    }

    return { width, height, dpr, ctx };
  };

  /**
   * Render sticky timeline header (Cycle ticks, Cycle labels, and Time labels) into Canvas context.
   * Pixel-aligned to perfectly mirror DOM-based header in Create Mode.
   */
  const renderTimelineHeader = (ctx: CanvasRenderingContext2D, options: RenderHeaderOptions) => {
    const { scrollLeft, width, height, totalCycles, cycleTimeMs, majorPx, styles } = options;

    // Background fill
    ctx.fillStyle = styles.surfaceHeader;
    ctx.fillRect(0, 0, width, height);

    // Calculate visible cycle range in current scroll viewport
    const startCycle = Math.max(0, Math.floor(scrollLeft / majorPx));
    const endCycle = Math.min(totalCycles - 1, Math.floor((scrollLeft + width) / majorPx));

    for (let c = startCycle; c <= endCycle; c++) {
      const x = c * majorPx - scrollLeft;
      const tickRightX = x + majorPx;

      // Draw right border line for current tick
      const lineX = Math.floor(tickRightX) - 0.5;
      ctx.strokeStyle = styles.border;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(lineX, 0);
      ctx.lineTo(lineX, height);
      ctx.stroke();

      // Draw text markers with exact DOM padding
      const textX = Math.floor(x) + HEADER_PADDING_LEFT;

      // 1. Cycle Label (Bold font, Top baseline aligned with DOM Flexbox vertical layout)
      ctx.fillStyle = styles.text;
      ctx.font = `bold ${styles.fontSizePx} ${styles.fontFamily}`;
      ctx.textBaseline = 'top';
      ctx.fillText(`Cycle ${c}`, textX, HEADER_LABEL_CYCLE_Y);

      // 2. Time Label (Dimmed text with opacity, top baseline Y)
      ctx.save();
      ctx.globalAlpha = HEADER_TEXT_DIM_ALPHA;
      ctx.fillStyle = styles.textDim;
      ctx.font = `${styles.fontSizePx} ${styles.fontFamily}`;
      ctx.textBaseline = 'top';
      ctx.fillText(`${c * cycleTimeMs}ms`, textX, HEADER_LABEL_TIME_Y);
      ctx.restore();
    }
  };

  /**
   * Render background grid surface and vertical time grid lines (Minor and Major) into Canvas context.
   * Culls grid lines outside the current scroll viewport for optimal rendering performance.
   */
  const renderBackgroundGrid = (ctx: CanvasRenderingContext2D, options: RenderGridOptions) => {
    const { scrollLeft, width, height, totalCycles, majorPx, minorPx, styles } = options;

    // Calculate grid division ratio dynamically (e.g. 400px / 40px = 10)
    const divisions = minorPx > 0 ? Math.round(majorPx / minorPx) : 10;

    // Background fill
    ctx.fillStyle = styles.surface;
    ctx.fillRect(0, 0, width, height);

    // Minor Grids
    const startMinor = Math.max(0, Math.floor(scrollLeft / minorPx));
    const endMinor = Math.min(totalCycles * divisions, Math.floor((scrollLeft + width) / minorPx));

    ctx.strokeStyle = styles.gridMinor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = startMinor; i <= endMinor; i++) {
      if (i % divisions === 0) continue; // Skip major grid lines
      const x = Math.floor(i * minorPx - scrollLeft) - 0.5;
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
    }
    ctx.stroke();

    // Major Grids (matching exact tick border X alignment)
    const startMajor = Math.max(0, Math.floor(scrollLeft / majorPx));
    const endMajor = Math.min(totalCycles, Math.floor((scrollLeft + width) / majorPx));

    ctx.strokeStyle = styles.gridMajor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = startMajor; i <= endMajor; i++) {
      const x = Math.floor(i * majorPx - scrollLeft) - 0.5;
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
    }
    ctx.stroke();
  };

  return {
    getThemeStyles,
    prepareCanvas,
    renderTimelineHeader,
    renderBackgroundGrid
  };
}
