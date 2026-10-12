import '@fontsource-variable/inter';
import './styles/app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import Popover from './Popover.svelte';
import { app } from './lib/state/app.svelte';
import { haptics } from './lib/haptics.svelte';
import { sounds } from './lib/sounds.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('Missing #app');

// Dev only: lets Playwright speed up the mock engine and read state. Stripped from builds.
if (import.meta.env.DEV) Object.assign(window, { __nhako: { app, haptics, sounds } });

// The tray popover is the same app in a small second window (src-tauri/src/popover.rs).
const popover = new URLSearchParams(location.search).get('view') === 'popover';
// Both windows share one stylesheet, so the popover's own page rules hang off this attribute.
if (popover) document.documentElement.dataset.view = 'popover';
export default mount(popover ? Popover : App, { target });
