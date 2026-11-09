// Shared bar for every draft page: page links, theme, reduced motion and the spec overlay.
// Choices are kept per browser only, as a convenience; pages work without storage.

const PAGES = [
  ['index.html', 'Overview'],
  ['01-icon-styles.html', 'Icon styles'],
  ['07-icon-set.html', 'Icon set'],
  ['02-brand-blue.html', 'Brand blue'],
  ['03-moments.html', 'Moments'],
  ['04-pages.html', 'Pages'],
  ['05-system-icons.html', 'System icons'],
  ['06-cover-mini.html', 'Cover screen'],
];

function load(key, fallback) {
  try { return localStorage.getItem('nb-draft-' + key) ?? fallback; } catch { return fallback; }
}
function save(key, value) {
  try { localStorage.setItem('nb-draft-' + key, value); } catch { /* storage blocked: fine */ }
}

const root = document.documentElement;
const params = new URLSearchParams(location.search);
const systemDark = matchMedia('(prefers-color-scheme: dark)').matches;
const systemReduced = matchMedia('(prefers-reduced-motion: reduce)').matches;

const state = {
  theme: params.get('theme') ?? load('theme', systemDark ? 'dark' : 'light'),
  reduced: params.get('reduced') ?? load('reduced', systemReduced ? 'on' : 'off'),
  spec: params.get('spec') ?? load('spec', 'off'),
};

function apply() {
  root.dataset.theme = state.theme;
  root.dataset.reduced = state.reduced;
  root.dataset.spec = state.spec;
  document.querySelectorAll('.switch').forEach((b) => {
    b.setAttribute('aria-pressed', String(state[b.dataset.key] === b.dataset.on));
  });
  window.dispatchEvent(new CustomEvent('draftchange', { detail: { ...state } }));
}

export function isReduced() { return state.reduced === 'on'; }
export function theme() { return state.theme; }

export function mountBar() {
  const here = location.pathname.split('/').pop() || 'index.html';
  const bar = document.createElement('header');
  bar.className = 'bar';
  bar.innerHTML = `
    <a class="home" href="index.html"><img src="../../desktop/public/favicon.svg" alt="">Nhako Beam drafts</a>
    <nav aria-label="Draft pages">${PAGES.map(([href, label]) =>
      `<a href="${href}"${href === here ? ' aria-current="page"' : ''}>${label}</a>`).join('')}</nav>
    <div class="switches">
      <button class="switch" data-key="theme" data-on="dark" data-off="light">Dark</button>
      <button class="switch" data-key="reduced" data-on="on" data-off="off">Reduced motion</button>
      <button class="switch" data-key="spec" data-on="on" data-off="off">Spec overlay</button>
    </div>`;
  document.body.prepend(bar);
  bar.querySelectorAll('.switch').forEach((b) => b.addEventListener('click', () => {
    const k = b.dataset.key;
    state[k] = state[k] === b.dataset.on ? b.dataset.off : b.dataset.on;
    save(k, state[k]);
    apply();
  }));
  apply();
}

apply();
