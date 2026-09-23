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
/**
 * Canvas Rendering Type Definitions
 */

import type { ActualCycle } from './analyze';

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

export interface RenderActualCycleLinesOptions {
  scrollLeft: number;
  width: number;
  height: number;
  actualCycles: ActualCycle[];
  pxPerMs: number;
  styles: ThemeStyles;
}
