import { hc } from 'hono/client';
import type { AppType } from '../../../api/src/index';

const API_BASE = import.meta.env.VITE_API_URL ?? '';

export const api = hc<AppType>(API_BASE);
