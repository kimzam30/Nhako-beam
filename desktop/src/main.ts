import '@fontsource-variable/inter';
import './styles/app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { app } from './lib/state/app.svelte';
import { haptics } from './lib/haptics.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('Missing #app');

// Dev only: lets Playwright speed up the mock engine and read state. Stripped from builds.
if (import.meta.env.DEV) Object.assign(window, { __nhako: { app, haptics } });

export default mount(App, { target });
