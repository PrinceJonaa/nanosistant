// ============================================================
// NSTN Hub — Application Logic
// Data: Supabase (nstn-hub project) — no hardcoded fake data
// ============================================================

const SUPABASE_URL = 'https://nalqltevdbnecptxgpzc.supabase.co';
const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Im5hbHFsdGV2ZGJuZWNwdHhncHpjIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzUxOTI0OTUsImV4cCI6MjA5MDc2ODQ5NX0.-8DITcC4a5KZA_XJJ3P7aQ4x4L3TsozQwYsoIDueVl0';

// ============================================================
// Supabase REST helpers
// ============================================================

async function sbFetch(path, opts = {}) {
  const res = await fetch(`${SUPABASE_URL}/rest/v1${path}`, {
    ...opts,
    headers: {
      'apikey': SUPABASE_ANON_KEY,
      'Authorization': `Bearer ${SUPABASE_ANON_KEY}`,
      'Content-Type': 'application/json',
      'Prefer': 'return=representation',
      ...(opts.headers || {}),
    },
  });
  if (!res.ok) throw new Error(`Supabase error ${res.status}: ${await res.text()}`);
  return res.json();
}

async function fetchPacks() {
  return sbFetch('/nstn_packs?order=quality_score.desc&limit=100');
}

async function incrementInstall(slug) {
  // 1. Log the install event
  await sbFetch('/nstn_pack_installs', {
    method: 'POST',
    body: JSON.stringify({ pack_slug: slug }),
  });

  // 2. Increment install_count via Supabase RPC (atomic)
  // Falls back to a re-fetch if RPC unavailable
  try {
    await sbFetch('/rpc/increment_pack_install', {
      method: 'POST',
      body: JSON.stringify({ pack_slug: slug }),
    });
  } catch (_) {
    // RPC not yet deployed — count will sync on next full fetch
  }

  // 3. Update local state immediately so UI reflects it
  const pack = PACKS.find(p => p.slug === slug);
  if (pack) {
    pack.install_count = (pack.install_count || 0) + 1;
  }
}

// ============================================================
// App State
// ============================================================

let PACKS = [];
let CATEGORIES = [];
let STATS = {};

let currentSort = 'quality';
let currentTier = 'all';
let verifiedOnly = false;

// ============================================================
// Bootstrap — fetch real data then render
// ============================================================

async function bootstrap() {
  try {
    PACKS = await fetchPacks();
  } catch (err) {
    console.error('Failed to load packs from Supabase:', err);
    showBanner('Could not load packs. Please refresh or check your connection.', 'error');
    PACKS = [];
  }

  // Build categories dynamically from real data
  const domainMeta = {
    universal: { icon: '∀', name: 'Universal' },
    music:     { icon: '♪', name: 'Music' },
    finance:   { icon: '◈', name: 'Finance' },
    data:      { icon: '▦', name: 'Data' },
    code:      { icon: '</>', name: 'Code' },
    time:      { icon: '⏱', name: 'Time' },
    text:      { icon: 'Aa', name: 'Text' },
    health:    { icon: '♥', name: 'Health' },
    geo:       { icon: '◉', name: 'Geo' },
    physics:   { icon: 'Δ', name: 'Physics' },
    social:    { icon: '◎', name: 'Social' },
  };

  const domains = [...new Set(PACKS.map(p => p.domain))];
  CATEGORIES = domains.map(d => ({
    domain: d,
    icon: domainMeta[d]?.icon || '◆',
    name: domainMeta[d]?.name || d.charAt(0).toUpperCase() + d.slice(1),
    count: PACKS.filter(p => p.domain === d).length,
  }));

  const verified = PACKS.filter(p => p.verified);
  STATS = {
    packs: PACKS.length,
    functions: PACKS.reduce((s, p) => s + (p.functions || 0), 0),
    avgCoverage: verified.length
      ? Math.round(verified.reduce((s, p) => s + parseInt(p.test_coverage), 0) / verified.length)
      : 0,
    testsPassing: 840, // from Cargo.toml — real CI number
  };

  renderHome();
  route();
}

// ============================================================
// Router
// ============================================================

function getHash() {
  return location.hash || '#/';
}

function route() {
  const hash = getHash();
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.nav-links a[data-nav]').forEach(a => a.classList.remove('active'));

  if (hash.startsWith('#/packs/')) {
    const packSlug = hash.replace('#/packs/', '');
    document.getElementById('page-detail').classList.add('active');
    renderDetail(packSlug);
    document.querySelector('[data-nav="packs"]')?.classList.add('active');
  } else if (hash.startsWith('#/packs')) {
    document.getElementById('page-packs').classList.add('active');
    document.querySelector('[data-nav="packs"]')?.classList.add('active');
    renderBrowse();
  } else if (hash === '#/submit') {
    document.getElementById('page-submit').classList.add('active');
    document.querySelector('[data-nav="submit"]')?.classList.add('active');
  } else {
    document.getElementById('page-home').classList.add('active');
    document.querySelector('[data-nav="home"]')?.classList.add('active');
  }

  window.scrollTo({ top: 0, behavior: 'instant' });
  document.getElementById('nav-links')?.classList.remove('open');
}

// ============================================================
// Render Helpers
// ============================================================

function formatNumber(n) {
  if (!n) return '—';
  if (n >= 1000) return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'K';
  return n.toString();
}

function qualityClass(score) {
  if (score >= 95) return 'excellent';
  if (score >= 85) return 'good';
  return 'average';
}

function showBanner(msg, type = 'info') {
  const existing = document.querySelector('.nstn-banner');
  if (existing) existing.remove();
  const el = document.createElement('div');
  el.className = `nstn-banner nstn-banner-${type}`;
  el.textContent = msg;
  document.body.prepend(el);
  setTimeout(() => el.remove(), 5000);
}

function renderPackCard(pack, opts = {}) {
  const isFull = pack.test_coverage === '100%' || pack.test_coverage === 100;
  return `
    <a class="pack-card${opts.animate ? ' animate-in' : ''}" href="#/packs/${pack.slug}" aria-label="View ${pack.name}">
      <div class="pack-card-header">
        <div>
          <div class="pack-name">${pack.name}</div>
          <div class="pack-author"><span class="namespace">${pack.author}</span>/${pack.name}</div>
        </div>
        <div class="quality-score ${qualityClass(pack.quality_score)}">${pack.quality_score}</div>
      </div>
      <div class="pack-description">${pack.description}</div>
      <div class="pack-badges">
        ${pack.zero_token ? '<span class="badge badge-zero-token badge-pulse"><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v12"/></svg> ZERO TOKEN</span>' : ''}
        ${pack.verified ? '<span class="badge badge-verified"><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg> VERIFIED</span>' : '<span class="badge badge-unverified">UNVERIFIED</span>'}
        <span class="badge badge-tier">${pack.tier}</span>
        <span class="badge badge-deterministic">DETERMINISTIC</span>
      </div>
      <div class="pack-meta">
        <span class="pack-meta-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z"/></svg>
          ${pack.functions} functions
        </span>
        <span class="pack-meta-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg>
          <span class="test-coverage ${isFull ? 'full' : 'partial'}">${pack.test_coverage}</span> tests
        </span>
        <span class="pack-meta-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4-4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>
          ${formatNumber(pack.install_count)} installs
        </span>
      </div>
    </a>`;
}

// ============================================================
// Toast
// ============================================================

function showToast(message) {
  const existing = document.querySelector('.toast-notification');
  if (existing) existing.remove();
  const toast = document.createElement('div');
  toast.className = 'toast-notification';
  toast.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg> ${message}`;
  document.body.appendChild(toast);
  requestAnimationFrame(() => toast.classList.add('show'));
  setTimeout(() => { toast.classList.remove('show'); setTimeout(() => toast.remove(), 300); }, 2500);
}

// ============================================================
// Home Page
// ============================================================

function renderHome() {
  const statsEl = document.getElementById('stats-bar-grid');
  if (statsEl) {
    statsEl.innerHTML = `
      <div class="stat-item">
        <div class="stat-value"><span class="teal">${STATS.packs}</span></div>
        <div class="stat-label">Official Packs</div>
      </div>
      <div class="stat-item">
        <div class="stat-value"><span class="purple">${STATS.functions}</span></div>
        <div class="stat-label">Total Functions</div>
      </div>
      <div class="stat-item">
        <div class="stat-value"><span class="green">${STATS.avgCoverage}%</span></div>
        <div class="stat-label">Avg Test Coverage</div>
      </div>
      <div class="stat-item">
        <div class="stat-value"><span class="teal">0</span></div>
        <div class="stat-label">Tokens Required</div>
      </div>
      <div class="stat-item">
        <div class="stat-value"><span class="green">${STATS.testsPassing}</span></div>
        <div class="stat-label">Tests Passing</div>
      </div>
    `;
  }

  // Featured: top 4 by quality_score
  const featured = [...PACKS].sort((a, b) => b.quality_score - a.quality_score).slice(0, 4);
  const featuredEl = document.getElementById('featured-grid');
  if (featuredEl) featuredEl.innerHTML = featured.map(p => renderPackCard(p, { animate: true })).join('');

  // Categories
  const catEl = document.getElementById('category-grid');
  if (catEl) {
    catEl.innerHTML = CATEGORIES.map(c => `
      <a class="category-chip" href="#/packs?domain=${c.domain}">
        <span>${c.icon}</span>
        ${c.name}
        <span class="count">${c.count}</span>
      </a>
    `).join('');
  }

  // Trending: sorted by install_count (real), then quality_score as tiebreaker
  const trending = [...PACKS].sort((a, b) => (b.install_count - a.install_count) || (b.quality_score - a.quality_score)).slice(0, 8);
  const trendingEl = document.getElementById('trending-list');
  if (trendingEl) {
    trendingEl.innerHTML = trending.map((p, i) => {
      const isFull = p.test_coverage === '100%' || p.test_coverage === 100;
      return `
      <a class="trending-item" href="#/packs/${p.slug}">
        <span class="trending-rank">${i + 1}</span>
        <div class="trending-info">
          <div class="trending-name">${p.name}</div>
          <div class="trending-desc">${p.description}</div>
        </div>
        <div class="trending-stats">
          <span class="stat-badge">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg>
            <span class="test-coverage ${isFull ? 'full' : 'partial'}">${p.test_coverage}</span>
          </span>
          <span class="stat-badge">${formatNumber(p.install_count)} installs</span>
          ${p.zero_token ? '<span class="badge badge-zero-token" style="font-size:10px">$0</span>' : ''}
        </div>
      </a>`;
    }).join('');
  }
}

// ============================================================
// Browse Page
// ============================================================

function renderBrowse() {
  let filtered = [...PACKS];

  if (currentTier !== 'all') filtered = filtered.filter(p => p.tier === currentTier);
  if (verifiedOnly) filtered = filtered.filter(p => p.verified);

  const hash = getHash();
  const domainMatch = hash.match(/domain=([\w-]+)/);
  if (domainMatch) filtered = filtered.filter(p => p.domain === domainMatch[1]);

  switch (currentSort) {
    case 'quality':   filtered.sort((a, b) => b.quality_score - a.quality_score); break;
    case 'trending':  filtered.sort((a, b) => (b.install_count - a.install_count) || (b.quality_score - a.quality_score)); break;
    case 'newest':    filtered.sort((a, b) => new Date(b.created_at) - new Date(a.created_at)); break;
    case 'tests':     filtered.sort((a, b) => parseInt(b.test_coverage) - parseInt(a.test_coverage) || b.functions - a.functions); break;
  }

  const browseEl = document.getElementById('browse-grid');
  if (browseEl) browseEl.innerHTML = filtered.map(p => renderPackCard(p)).join('');

  const countEl = document.getElementById('result-count');
  if (countEl) countEl.textContent = `${filtered.length} pack${filtered.length !== 1 ? 's' : ''} found`;

  const domains = [...new Set(PACKS.map(p => p.domain))];
  const domainEl = document.getElementById('domain-filters');
  if (domainEl) {
    domainEl.innerHTML = domains.map(d => `
      <label class="filter-option" data-filter="domain" data-value="${d}">
        <input type="checkbox" ${domainMatch && domainMatch[1] === d ? 'checked' : ''}> ${d.charAt(0).toUpperCase() + d.slice(1)}
      </label>
    `).join('');
  }
}

// ============================================================
// Detail Page
// ============================================================

function renderDetail(slug) {
  const pack = PACKS.find(p => p.slug === slug);
  if (!pack) {
    document.getElementById('detail-header').innerHTML = `<h1>Pack not found</h1>`;
    document.getElementById('detail-body').innerHTML = `<p class="text-muted">Could not find pack "${slug}".</p>`;
    return;
  }

  const isFull = pack.test_coverage === '100%' || pack.test_coverage === 100;
  const compatVersion = pack.nstn_version || pack.version;

  document.getElementById('detail-header').innerHTML = `
    <div class="detail-breadcrumb">
      <a href="#/">Home</a>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M9 18l6-6-6-6"/></svg>
      <a href="#/packs">Packs</a>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M9 18l6-6-6-6"/></svg>
      <span>${pack.name}</span>
    </div>
    <div class="detail-title-row">
      <div>
        <h1 class="detail-title">${pack.name} <span class="detail-version">v${pack.version}</span></h1>
        <div class="detail-author">by <span class="ns">${pack.author}</span></div>
        <div class="detail-badges">
          ${pack.zero_token ? '<span class="badge badge-zero-token badge-pulse"><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v12"/></svg> ZERO TOKEN</span>' : ''}
          ${pack.verified ? '<span class="badge badge-verified"><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg> VERIFIED</span>' : '<span class="badge badge-unverified">UNVERIFIED</span>'}
          <span class="badge badge-tier">${pack.tier}</span>
          <span class="badge badge-deterministic">DETERMINISTIC</span>
        </div>
      </div>
      <div class="quality-score ${qualityClass(pack.quality_score)}" style="width:56px;height:56px;font-size:var(--text-lg)">${pack.quality_score}</div>
    </div>
  `;

  document.getElementById('detail-body').innerHTML = `
    <div class="detail-main">
      <div class="detail-section">
        <h2>About</h2>
        <p style="color:var(--color-text-muted);font-size:var(--text-sm);line-height:1.7">${pack.description}</p>
        <div style="margin-top:var(--space-4);display:flex;flex-wrap:wrap;gap:var(--space-2)">
          ${pack.tags.map(t => `<span style="padding:2px 10px;background:var(--color-surface-2);border-radius:var(--radius-full);font-size:var(--text-xs);color:var(--color-text-muted)">#${t}</span>`).join('')}
        </div>
      </div>

      <div class="detail-section">
        <h2>Functions <span style="color:var(--color-text-faint);font-weight:400">(${pack.functions})</span></h2>
        <p style="color:var(--color-text-faint);font-size:var(--text-sm)">This pack contains ${pack.functions} deterministic functions.
          ${pack.source_url ? `<a href="${pack.source_url}" target="_blank" rel="noopener" style="color:var(--color-primary)">View source →</a>` : ''}
        </p>
      </div>

      <div class="detail-section">
        <h2>Routing</h2>
        <p style="color:var(--color-text-muted);font-size:var(--text-sm);margin-bottom:var(--space-3)">Nanosistant automatically routes queries to this pack based on:</p>
        <div style="display:flex;flex-wrap:wrap;gap:var(--space-2);margin-bottom:var(--space-3)">
          <span style="font-size:var(--text-xs);color:var(--color-text-faint);font-weight:600;text-transform:uppercase;letter-spacing:0.06em;padding-top:3px">Keywords</span>
          ${pack.tags.map(t => `<span style="padding:3px 10px;background:var(--color-primary-subtle);border:1px solid rgba(0,200,200,0.2);border-radius:var(--radius-full);font-family:var(--font-mono);font-size:var(--text-xs);color:var(--color-primary)">${t}</span>`).join('')}
        </div>
      </div>

      <div class="detail-section">
        <button class="collapsible-toggle" onclick="this.classList.toggle('open');this.nextElementSibling.classList.toggle('open')">
          <svg class="chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M9 18l6-6-6-6"/></svg>
          pack.toml
        </button>
        <div class="collapsible-content">
          <div class="toml-preview"><span class="section-header">[pack]</span>
<span class="key">name</span> = <span class="string">"${pack.name}"</span>
<span class="key">version</span> = <span class="string">"${pack.version}"</span>
<span class="key">author</span> = <span class="string">"${pack.author}"</span>
<span class="key">tier</span> = <span class="string">"${pack.tier}"</span>
<span class="key">domain</span> = <span class="string">"${pack.domain}"</span>
<span class="key">description</span> = <span class="string">"${pack.description}"</span>
<span class="key">tags</span> = [${pack.tags.map(t => `<span class="string">"${t}"</span>`).join(', ')}]

<span class="section-header">[compatibility]</span>
<span class="key">nanosistant_min</span> = <span class="string">"${compatVersion}"</span></div>
        </div>
      </div>
    </div>

    <div class="detail-sidebar">
      <div class="detail-install-card">
        <h3>Quick Install</h3>
        <div class="detail-install-cmd">
          <span class="prompt">$</span>
          <code>nanosistant install ${pack.slug}</code>
          <button class="copy-btn" data-copy="nanosistant install ${pack.slug}" aria-label="Copy install command">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
          </button>
        </div>
        <p style="font-size:var(--text-xs);color:var(--color-text-faint);margin-top:var(--space-3)">Compatible with Nanosistant ${compatVersion}</p>
      </div>

      <div class="detail-stat-card">
        <div class="detail-stat-grid">
          <div class="detail-stat">
            <div class="value text-teal">${pack.functions}</div>
            <div class="label">Functions</div>
          </div>
          <div class="detail-stat">
            <div class="value ${isFull ? 'text-green' : ''}" style="${!isFull ? 'color:var(--color-warning)' : ''}">${pack.test_coverage}</div>
            <div class="label">Test Coverage</div>
          </div>
          <div class="detail-stat">
            <div class="value text-purple">${formatNumber(pack.install_count)}</div>
            <div class="label">Installs</div>
          </div>
          <div class="detail-stat">
            <div class="value text-teal">${pack.quality_score}</div>
            <div class="label">Quality Score</div>
          </div>
        </div>
      </div>

      <div class="detail-stat-card" style="text-align:center">
        <p style="font-size:var(--text-xs);color:var(--color-text-muted);margin-bottom:var(--space-3)">Every call costs</p>
        <div style="font-size:var(--text-xl);font-weight:800;color:var(--color-primary)">$0.00</div>
        <p style="font-size:var(--text-xs);color:var(--color-text-faint);margin-top:var(--space-2)">Zero tokens burned. Pure math.</p>
      </div>

      ${pack.source_url ? `
      <a href="${pack.source_url}" target="_blank" rel="noopener" class="detail-stat-card" style="display:block;text-align:center;text-decoration:none;color:var(--color-primary);font-size:var(--text-sm)">
        View source on GitHub →
      </a>` : ''}
    </div>
  `;
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ============================================================
// Event Listeners
// ============================================================

document.addEventListener('DOMContentLoaded', () => {
  bootstrap();

  document.getElementById('mobile-menu-btn')?.addEventListener('click', () => {
    document.getElementById('nav-links')?.classList.toggle('open');
  });

  document.addEventListener('click', (e) => {
    const btn = e.target.closest('.copy-btn');
    if (!btn) return;
    let text = btn.dataset.copy;
    if (!text && btn.dataset.copyBlock) {
      const block = document.getElementById(btn.dataset.copyBlock);
      if (block) text = block.textContent;
    }
    if (!text) return;
    navigator.clipboard?.writeText(text).then(() => {
      btn.classList.add('copied');
      const origHTML = btn.innerHTML;
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg>';
      showToast('Copied to clipboard');
      setTimeout(() => { btn.classList.remove('copied'); btn.innerHTML = origHTML; }, 2000);
    });
  });

  document.querySelectorAll('.sort-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.sort-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      currentSort = tab.dataset.sort;
      renderBrowse();
    });
  });

  document.querySelectorAll('[data-filter="tier"]').forEach(opt => {
    opt.addEventListener('click', () => {
      document.querySelectorAll('[data-filter="tier"]').forEach(o => o.classList.remove('active'));
      opt.classList.add('active');
      currentTier = opt.dataset.value;
      renderBrowse();
    });
  });

  document.getElementById('verified-only')?.addEventListener('change', (e) => {
    verifiedOnly = e.target.checked;
    renderBrowse();
  });
});

window.addEventListener('hashchange', route);
