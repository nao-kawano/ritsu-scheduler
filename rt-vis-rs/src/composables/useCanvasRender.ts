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
import type { ActualCycle } from '../types/analyze';

export interface ThemeStyles {
  textColor: string;
  textDimColor: string;
  borderColor: string;
  surfaceHeaderColor: string;
  surfaceColor: string;
  gridMajorColor: string;
  gridMinorColor: string;
  primaryColor: string;
  accentColor: string;
  errorColor: string;
  warningColor: string;
  fontSizePx: string;
  fontFamily: string;
  statusNormalColor: string;
  statusOverrunColor: string;
  statusSkipColor: string;
  statusIncompleteColor: string;
  eventReadyColor: string;
  eventExitColor: string;
  eventOverrunColor: string;
  eventErrorColor: string;
  eventSkipColor: string;
  eventLateColor: string;
  eventRetransmitColor: string;
}

export interface RenderHeaderOptions {
  scrollLeft: number;
  width: number;
  height: number;
  totalCycles: number;
  cycleTimeMs: number;
  majorPx: number;
  styles: ThemeStyles;
  actualCycles?: ActualCycle[];
  pxPerMs?: number;
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
const HEADER_PADDING_LEFT = (8 + 1);
const HEADER_LABEL_CYCLE_Y = 6;
const HEADER_LABEL_TIME_Y = 21;
const HEADER_TEXT_DIM_ALPHA = 0.8;

export interface RenderActualCycleLinesOptions {
  scrollLeft: number;
  width: number;
  height: number;
  actualCycles: ActualCycle[];
  pxPerMs: number;
  styles: ThemeStyles;
}

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
    const getVar = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;

    return {
      textColor: getVar('--rt-color-text', 'rgba(0, 0, 0, 0.87)'),
      textDimColor: getVar('--rt-color-text-dim', 'rgba(0, 0, 0, 0.6)'),
      borderColor: getVar('--rt-color-border', 'rgba(0, 0, 0, 0.12)'),
      surfaceHeaderColor: getVar('--rt-color-surface-header', '#f5f5f5'),
      surfaceColor: getVar('--rt-color-surface', '#ffffff'),
      gridMajorColor: getVar('--rt-grid-major', 'rgba(0, 0, 0, 0.3)'),
      gridMinorColor: getVar('--rt-grid-minor', 'rgba(0, 0, 0, 0.1)'),
      primaryColor: getVar('--rt-color-primary', '#415F91'),
      accentColor: getVar('--rt-color-accent', '#6f5575'),
      errorColor: getVar('--rt-color-error', '#ba1a1a'),
      warningColor: getVar('--rt-color-warning', '#e6a23c'),
      fontSizePx: getVar('--rt-font-xs', '11px'),
      fontFamily: style.fontFamily?.trim() || getVar('--rt-font-family', 'Inter, system-ui, sans-serif'),
      statusNormalColor: getVar('--rt-color-status-normal', '#705575'),
      statusOverrunColor: getVar('--rt-color-status-overrun', '#ba1a1a'),
      statusSkipColor: getVar('--rt-color-status-skip', '#f5e389'),
      statusIncompleteColor: getVar('--rt-color-status-incomplete', '#74777f'),
      eventReadyColor: getVar('--rt-color-event-ready', '#415F91'),
      eventExitColor: getVar('--rt-color-event-exit', '#74777f'),
      eventOverrunColor: getVar('--rt-color-event-overrun', '#ba1a1a'),
      eventErrorColor: getVar('--rt-color-event-error', '#ba1a1a'),
      eventSkipColor: getVar('--rt-color-event-skip', '#f5e389'),
      eventLateColor: getVar('--rt-color-event-late', '#f5e389'),
      eventRetransmitColor: getVar('--rt-color-event-retransmit', '#705575')
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
   * Render sticky timeline header into Canvas context.
   * Renders cycle boundary lines and time labels at actual start_ms locations.
   */
  const renderTimelineHeader = (ctx: CanvasRenderingContext2D, options: RenderHeaderOptions) => {
    const { scrollLeft, width, height, totalCycles, cycleTimeMs, majorPx, styles, actualCycles, pxPerMs } = options;

    const safeMajorPx = Math.max(1, majorPx || 0);
    const safeCycleTimeMs = Math.max(0, cycleTimeMs || 0);
    const safePxPerMs = pxPerMs && pxPerMs > 0 ? pxPerMs : safeMajorPx / (safeCycleTimeMs || 1);

    // Background fill
    ctx.save();
    {
      ctx.fillStyle = styles.surfaceHeaderColor;
      ctx.fillRect(0, 0, width, height);
    }
    ctx.restore();

    // Render actual cycle boundary lines & labels if actualCycles dataset is present
    if (actualCycles && actualCycles.length > 0) {
      const marginMs = safeCycleTimeMs * 2;
      const startMs = Math.max(0, scrollLeft / safePxPerMs - marginMs);
      const endMs = (scrollLeft + width) / safePxPerMs + marginMs;

      actualCycles.forEach(ac => {
        if (ac.start_ms < startMs || ac.start_ms > endMs) return;

        const x = Math.floor(ac.start_ms * safePxPerMs - scrollLeft) - 0.5;

        ctx.save();
        {
          // Draw actual cycle boundary line
          ctx.strokeStyle = styles.borderColor;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(x, 0);
          ctx.lineTo(x, height);
          ctx.stroke();

          // Draw text markers with exact DOM padding
          const textX = Math.floor(x) + HEADER_PADDING_LEFT;

          // Cycle Label (Bold font)
          ctx.fillStyle = styles.textColor;
          ctx.font = `bold ${styles.fontSizePx} ${styles.fontFamily}`;
          ctx.textBaseline = 'top';
          ctx.fillText(`Cycle ${ac.cycle}`, textX, HEADER_LABEL_CYCLE_Y);

          // Actual Time Label
          const formattedTime = Number.isInteger(ac.start_ms) ? `${ac.start_ms}ms` : `${ac.start_ms.toFixed(1)}ms`;
          ctx.save();
          {
            ctx.globalAlpha = HEADER_TEXT_DIM_ALPHA;
            ctx.fillStyle = styles.textDimColor;
            ctx.font = `${styles.fontSizePx} ${styles.fontFamily}`;
            ctx.textBaseline = 'top';
            ctx.fillText(formattedTime, textX, HEADER_LABEL_TIME_Y);
          }
          ctx.restore();
        }
        ctx.restore();
      });
      return;
    }

    // Fallback: render planned cycle headers when actual log is not yet loaded
    const startCycle = Math.max(0, Math.floor(scrollLeft / safeMajorPx));
    const endCycle = Math.min(totalCycles - 1, Math.floor((scrollLeft + width) / safeMajorPx));

    for (let c = startCycle; c <= endCycle; c++) {
      const x = Math.floor(c * safeMajorPx - scrollLeft) - 0.5;

      ctx.save();
      {
        ctx.strokeStyle = styles.borderColor;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();

        const textX = Math.floor(x) + HEADER_PADDING_LEFT;

        ctx.fillStyle = styles.textColor;
        ctx.font = `bold ${styles.fontSizePx} ${styles.fontFamily}`;
        ctx.textBaseline = 'top';
        ctx.fillText(`Cycle ${c}`, textX, HEADER_LABEL_CYCLE_Y);

        const plannedMs = c * safeCycleTimeMs;
        ctx.save();
        {
          ctx.globalAlpha = HEADER_TEXT_DIM_ALPHA;
          ctx.fillStyle = styles.textDimColor;
          ctx.font = `${styles.fontSizePx} ${styles.fontFamily}`;
          ctx.textBaseline = 'top';
          ctx.fillText(`${plannedMs}ms`, textX, HEADER_LABEL_TIME_Y);
        }
        ctx.restore();
      }
      ctx.restore();
    }
  };

  /**
   * Render background grid surface and vertical time grid lines (Minor and Major) into Canvas context.
   * Culls grid lines outside the current scroll viewport for optimal rendering performance.
   */
  const renderBackgroundGrid = (ctx: CanvasRenderingContext2D, options: RenderGridOptions) => {
    const { scrollLeft, width, height, majorPx, minorPx, styles } = options;

    const safeMajorPx = Math.max(1, majorPx || 0);
    const safeMinorPx = Math.max(0.1, minorPx || 0);
    const divisions = Math.max(1, Math.round(safeMajorPx / safeMinorPx));

    // Background fill
    ctx.save();
    {
      ctx.fillStyle = styles.surfaceColor;
      ctx.fillRect(0, 0, width, height);
    }
    ctx.restore();

    // Minor Grids
    const startMinor = Math.max(0, Math.floor(scrollLeft / safeMinorPx));
    const endMinor = Math.floor((scrollLeft + width) / safeMinorPx);

    ctx.save();
    {
      ctx.strokeStyle = styles.gridMinorColor;
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (let i = startMinor; i <= endMinor; i++) {
        if (i % divisions === 0) continue; // Skip major grid lines
        const x = Math.floor(i * safeMinorPx - scrollLeft) - 0.5;
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
      }
      ctx.stroke();
    }
    ctx.restore();

    // Major Grids (matching exact tick border X alignment)
    const startMajor = Math.max(0, Math.floor(scrollLeft / safeMajorPx));
    const endMajor = Math.floor((scrollLeft + width) / safeMajorPx);

    ctx.save();
    {
      ctx.strokeStyle = styles.gridMajorColor;
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (let i = startMajor; i <= endMajor; i++) {
        const x = Math.floor(i * safeMajorPx - scrollLeft) - 0.5;
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
      }
      ctx.stroke();
    }
    ctx.restore();
  };

  /**
   * Render actual cycle start lines (thin accent lines overlaid on background grid).
   */
  const renderActualCycleLines = (ctx: CanvasRenderingContext2D, options: RenderActualCycleLinesOptions) => {
    const { scrollLeft, width, height, actualCycles, pxPerMs, styles } = options;
    if (!actualCycles || actualCycles.length === 0 || pxPerMs <= 0) return;

    ctx.save();
    {
      ctx.strokeStyle = styles.primaryColor;
      ctx.globalAlpha = 0.35;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      actualCycles.forEach(ac => {
        const x = Math.floor(ac.start_ms * pxPerMs - scrollLeft) - 0.5;
        if (x >= -2 && x <= width + 2) {
          ctx.moveTo(x, 0);
          ctx.lineTo(x, height);
        }
      });
      ctx.stroke();
    }
    ctx.restore();
  };

  return {
    getThemeStyles,
    prepareCanvas,
    renderTimelineHeader,
    renderBackgroundGrid,
    renderActualCycleLines
  };
}
