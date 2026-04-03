// ============================================================
// NSTN Hub — Application Logic
// ============================================================

// Pack data — full 16-pack dataset
const PACKS = [
  // UNIVERSAL TIER
  {
    name: "logic-set-theory", slug: "logic-set-theory", author: "nanosistant",
    tier: "Universal", domain: "universal", version: "0.7.0",
    description: "Boolean truth tables, set operations, combinations, permutations, propositional logic evaluator",
    functions: 17, test_coverage: 100, quality_score: 99, usage_count: 5102,
    verified: true, tags: ["logic","math","sets","combinatorics"], zero_token: true,
    nstn_version: ">=0.6.0",
    functions_list: [
      { name: "truth_table", sig: "truth_table(expr, vars) -> Vec<Vec<bool>>", desc: "Evaluate a boolean expression over all variable combinations", example: 'truth_table("A && B", ["A","B"]) → [[F,F,F],[F,T,F],[T,F,F],[T,T,T]]' },
      { name: "set_union", sig: "set_union(a: &[i64], b: &[i64]) -> Vec<i64>", desc: "Set union", example: 'set_union([1,2,3],[2,3,4]) → [1,2,3,4]' },
      { name: "combinations", sig: "combinations(n: u64, k: u64) -> u64", desc: "C(n,k) — n choose k", example: 'combinations(10, 3) → 120' },
      { name: "powerset_size", sig: "powerset_size(n: usize) -> usize", desc: "Number of subsets of a set of size n", example: 'powerset_size(4) → 16' }
    ]
  },
  {
    name: "graph-theory", slug: "graph-theory", author: "nanosistant",
    tier: "Universal", domain: "universal", version: "0.7.0",
    description: "Degree centrality, clustering coefficient, Dunbar layers, Erdős–Rényi threshold, small-world detection",
    functions: 12, test_coverage: 100, quality_score: 98, usage_count: 3841,
    verified: true, tags: ["graph","networks","math","social"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  {
    name: "information-theory", slug: "information-theory", author: "nanosistant",
    tier: "Universal", domain: "universal", version: "0.7.0",
    description: "Shannon entropy, KL divergence, Levenshtein distance, channel capacity, compression ratio, Hamming distance",
    functions: 11, test_coverage: 100, quality_score: 98, usage_count: 3201,
    verified: true, tags: ["information","entropy","compression","distance"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  {
    name: "probability-stats", slug: "probability-stats", author: "nanosistant",
    tier: "Universal", domain: "universal", version: "0.7.0",
    description: "Bayes theorem, binomial/Poisson/normal distributions, confidence intervals, precision/recall/F1",
    functions: 15, test_coverage: 100, quality_score: 97, usage_count: 4102,
    verified: true, tags: ["probability","bayes","statistics","ml"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  // DOMAIN TIER
  {
    name: "music-theory-core", slug: "music-theory-core", author: "nanosistant",
    tier: "Domain", domain: "music", version: "0.7.0",
    description: "Scale degrees (14 modes), chord-to-roman, BPM math, tempo feels, delay times, rhyme detection, RT60",
    functions: 22, test_coverage: 100, quality_score: 98, usage_count: 12847,
    verified: true, tags: ["music","theory","production","bpm"], zero_token: true, nstn_version: ">=0.6.0",
    functions_list: [
      { name: "scale_degrees", sig: "scale_degrees(key: &str, mode: &str) -> Vec<String>", desc: "14 modes: major, minor, dorian, phrygian, lydian, mixolydian, blues, pentatonic…", example: 'scale_degrees("C", "blues") → ["C","Eb","F","F#","G","Bb"]' },
      { name: "bpm_to_bar_duration", sig: "bpm_to_bar_duration(bpm: u32, beats_per_bar: u32) -> f64", desc: "Bar duration in seconds", example: 'bpm_to_bar_duration(140, 4) → 1.714s' },
      { name: "delay_times", sig: "delay_times(bpm: u32) -> DelayTimes", desc: "Quarter, eighth, dotted eighth, sixteenth, triplet ms values", example: 'delay_times(120) → { quarter: 500ms, eighth: 250ms, dotted_eighth: 375ms }' },
      { name: "chord_to_roman", sig: "chord_to_roman(chord: &str, key: &str) -> String", desc: "Convert chord name to roman numeral in a given key", example: 'chord_to_roman("Am", "C") → "vi"' },
      { name: "tempo_feel", sig: "tempo_feel(bpm: u32) -> &str", desc: "Classical tempo term", example: 'tempo_feel(120) → "Allegro (fast, lively)"' }
    ]
  },
  {
    name: "finance-quant", slug: "finance-quant", author: "nanosistant",
    tier: "Domain", domain: "finance", version: "0.7.0",
    description: "Black-Scholes, Kelly criterion, RSI/MACD/EMA/SMA, Sharpe, Sortino, max drawdown, VaR, position sizing",
    functions: 24, test_coverage: 100, quality_score: 97, usage_count: 9234,
    verified: true, tags: ["finance","options","trading","risk"], zero_token: true, nstn_version: ">=0.6.0",
    functions_list: [
      { name: "black_scholes_call", sig: "black_scholes_call(s, k, t, r, sigma) -> f64", desc: "Black-Scholes call option price", example: 'black_scholes_call(100,100,1,0,0.2) → 7.97' },
      { name: "kelly_fraction", sig: "kelly_fraction(win_rate: f64, win_loss_ratio: f64) -> f64", desc: "Optimal bet fraction from Kelly criterion", example: 'kelly_fraction(0.55, 1.5) → 0.25' },
      { name: "sharpe_ratio", sig: "sharpe_ratio(ret: f64, rf: f64, vol: f64) -> f64", desc: "Risk-adjusted return", example: 'sharpe_ratio(0.15, 0.05, 0.20) → 0.5' },
      { name: "max_drawdown", sig: "max_drawdown(prices: &[f64]) -> f64", desc: "Maximum peak-to-trough drawdown", example: 'max_drawdown([100,90,80,100]) → 0.2 (20%)' }
    ]
  },
  {
    name: "code-utilities", slug: "code-utilities", author: "nanosistant",
    tier: "Domain", domain: "code", version: "0.7.0",
    description: "Semver parse/compare/satisfy/bump, base64, hex, DJB2/FNV1a hashing, diff stats, cyclomatic complexity",
    functions: 16, test_coverage: 100, quality_score: 97, usage_count: 8201,
    verified: true, tags: ["code","semver","hash","encoding"], zero_token: true, nstn_version: ">=0.6.0",
    functions_list: [
      { name: "semver_satisfies", sig: 'semver_satisfies(version, constraint) -> Result<bool>', desc: 'Check ^, ~, >=, <=, = constraints', example: 'semver_satisfies("1.3.0", "^1.2.0") → true' },
      { name: "semver_bump", sig: 'semver_bump(version, bump) -> Result<String>', desc: 'Increment major/minor/patch', example: 'semver_bump("1.2.3", "minor") → "1.3.0"' },
      { name: "base64_encode", sig: 'base64_encode(input: &[u8]) -> String', desc: 'Standard base64 encoding', example: 'base64_encode(b"Man") → "TWFu"' },
      { name: "diff_stats", sig: 'diff_stats(a: &str, b: &str) -> DiffStats', desc: 'Line-level diff: additions, deletions, unchanged', example: 'diff_stats(old, new) → { additions: 3, deletions: 1, unchanged: 10 }' }
    ]
  },
  {
    name: "data-analysis", slug: "data-analysis", author: "nanosistant",
    tier: "Domain", domain: "data", version: "0.7.0",
    description: "Describe, percentiles, correlation, linear regression, rolling stats, normalization, outlier detection",
    functions: 18, test_coverage: 100, quality_score: 96, usage_count: 7521,
    verified: true, tags: ["data","statistics","analysis","pandas"], zero_token: true, nstn_version: ">=0.6.0",
    functions_list: [
      { name: "describe", sig: "describe(data: &[f64]) -> Option<DescribeResult>", desc: "Full descriptive stats: mean, std, min, p25, median, p75, p90, p99, max, skewness", example: 'describe([1,2,3,4,5]) → { mean: 3.0, median: 3.0, std: 1.41 }' },
      { name: "linear_regression", sig: "linear_regression(x, y) -> Option<(slope, intercept, r2)>", desc: "OLS linear regression with R²", example: 'linear_regression([1,2,3],[2,4,6]) → (2.0, 0.0, 1.0)' },
      { name: "correlation", sig: "correlation(x: &[f64], y: &[f64]) -> Option<f64>", desc: "Pearson correlation coefficient", example: 'correlation([1,2,3],[3,6,9]) → 1.0' }
    ]
  },
  {
    name: "datetime-calendar", slug: "datetime-calendar", author: "nanosistant",
    tier: "Domain", domain: "time", version: "0.7.0",
    description: "Business days, leap year, ISO week, quarter, duration format, timezone offsets, age calculation",
    functions: 14, test_coverage: 100, quality_score: 96, usage_count: 6102,
    verified: true, tags: ["time","calendar","dates","timezone"], zero_token: true, nstn_version: ">=0.6.0", functions_list: []
  },
  {
    name: "text-processing", slug: "text-processing", author: "nanosistant",
    tier: "Domain", domain: "text", version: "0.7.0",
    description: "Flesch reading ease, keyword extraction, keyword density, slugify, sentiment signals, lexical diversity",
    functions: 13, test_coverage: 100, quality_score: 95, usage_count: 5891,
    verified: true, tags: ["text","nlp","readability","keywords"], zero_token: true, nstn_version: ">=0.6.0", functions_list: []
  },
  {
    name: "geo-spatial", slug: "geo-spatial", author: "nanosistant",
    tier: "Domain", domain: "geo", version: "0.7.0",
    description: "Haversine distance, midpoint, bounding box, bearing, DD/DMS conversion, timezone from longitude",
    functions: 10, test_coverage: 100, quality_score: 96, usage_count: 2891,
    verified: true, tags: ["geo","distance","coordinates","maps"], zero_token: true, nstn_version: ">=0.6.0", functions_list: []
  },
  {
    name: "physics-fundamentals", slug: "physics-fundamentals", author: "nanosistant",
    tier: "Domain", domain: "physics", version: "0.7.0",
    description: "Kinematics, energy/work/power, wave/frequency, thermodynamics, Ohm's law, dB conversion",
    functions: 17, test_coverage: 100, quality_score: 95, usage_count: 2341,
    verified: true, tags: ["physics","mechanics","waves","thermodynamics"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  {
    name: "health-metrics", slug: "health-metrics", author: "nanosistant",
    tier: "Domain", domain: "health", version: "0.7.0",
    description: "BMI, BMR (Mifflin-St Jeor), TDEE, heart rate zones, VO2 max, macro calculations, recovery score",
    functions: 12, test_coverage: 100, quality_score: 95, usage_count: 4821,
    verified: true, tags: ["health","fitness","nutrition","wellness"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  {
    name: "social-dynamics", slug: "social-dynamics", author: "nanosistant",
    tier: "Domain", domain: "social", version: "0.7.0",
    description: "Viral coefficient, NPS, LTV, CAC, churn/retention, Metcalfe's law, diffusion of innovations",
    functions: 12, test_coverage: 100, quality_score: 94, usage_count: 1921,
    verified: true, tags: ["social","growth","saas","network"], zero_token: true, nstn_version: ">=0.7.0", functions_list: []
  },
  // OPERATOR / COMMUNITY
  {
    name: "jersey-club-production", slug: "jersey-club-production", author: "PrinceJonaa",
    tier: "Operator", domain: "music", version: "0.1.0",
    description: "BPM ranges (140-160), shuffle ratios, chant density, hi-hat patterns for Jersey Club production",
    functions: 8, test_coverage: 87, quality_score: 84, usage_count: 412,
    verified: false, tags: ["music","jersey-club","production","dance"], zero_token: true, nstn_version: ">=0.6.0", functions_list: []
  },
  {
    name: "arabic-maqam-scales", slug: "arabic-maqam-scales", author: "community",
    tier: "Operator", domain: "music", version: "0.1.0",
    description: "Quarter-tone intervals, maqam scale patterns, ajnas classification, modulation rules",
    functions: 9, test_coverage: 91, quality_score: 78, usage_count: 187,
    verified: false, tags: ["music","arabic","maqam","microtonal"], zero_token: true, nstn_version: ">=0.6.0", functions_list: []
  }
];

// Computed stats
const STATS = {
  packs: PACKS.length,  // all 16 packs
  totalPacks: PACKS.length,
  functions: 211,  // canonical display count per spec
  avgCoverage: Math.round(PACKS.filter(p => p.verified).reduce((s, p) => s + p.test_coverage, 0) / PACKS.filter(p => p.verified).length),
  testsPassing: 840,
  zeroTokenCalls: PACKS.reduce((s, p) => s + p.usage_count, 0),
};

const CATEGORIES = [
  { name: "Universal", icon: "∀", domain: "universal" },
  { name: "Music", icon: "♪", domain: "music" },
  { name: "Finance", icon: "◈", domain: "finance" },
  { name: "Data", icon: "▦", domain: "data" },
  { name: "Code", icon: "</>", domain: "code" },
  { name: "Time", icon: "⏱", domain: "time" },
  { name: "Text", icon: "Aa", domain: "text" },
  { name: "Health", icon: "♥", domain: "health" },
  { name: "Geo", icon: "◉", domain: "geo" },
  { name: "Physics", icon: "Δ", domain: "physics" },
  { name: "Social", icon: "◎", domain: "social" },
];

// Compute category counts
CATEGORIES.forEach(c => {
  c.count = PACKS.filter(p => p.domain === c.domain).length;
});

// ============================================================
// Router
// ============================================================

function getHash() {
  return location.hash || '#/';
}

function route() {
  const hash = getHash();
  const pages = document.querySelectorAll('.page');
  pages.forEach(p => p.classList.remove('active'));

  // Update nav
  document.querySelectorAll('.nav-links a[data-nav]').forEach(a => a.classList.remove('active'));

  if (hash.startsWith('#/packs/')) {
    const packName = hash.replace('#/packs/', '');
    document.getElementById('page-detail').classList.add('active');
    renderDetail(packName);
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
  // Close mobile menu
  document.getElementById('nav-links')?.classList.remove('open');
}

// ============================================================
// Render Helpers
// ============================================================

function formatNumber(n) {
  if (n >= 1000) return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'K';
  return n.toString();
}

function qualityClass(score) {
  if (score >= 95) return 'excellent';
  if (score >= 85) return 'good';
  return 'average';
}

function coverageDisplay(cov) {
  return typeof cov === 'number' ? cov + '%' : cov;
}

function renderPackCard(pack, opts = {}) {
  const covStr = coverageDisplay(pack.test_coverage);
  const isFull = pack.test_coverage === 100 || pack.test_coverage === '100%';
  return `
    <a class="pack-card${opts.animate ? ' animate-in' : ''}" href="#/packs/${pack.name}" aria-label="View ${pack.name}">
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
          <span class="test-coverage ${isFull ? 'full' : 'partial'}">${covStr}</span> tests
        </span>
        <span class="pack-meta-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4-4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>
          ${formatNumber(pack.usage_count)}
        </span>
      </div>
    </a>`;
}

// ============================================================
// Toast notification
// ============================================================

function showToast(message) {
  // Remove any existing toast
  const existing = document.querySelector('.toast-notification');
  if (existing) existing.remove();

  const toast = document.createElement('div');
  toast.className = 'toast-notification';
  toast.innerHTML = `
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg>
    ${message}
  `;
  document.body.appendChild(toast);

  // Trigger animation
  requestAnimationFrame(() => {
    toast.classList.add('show');
  });

  setTimeout(() => {
    toast.classList.remove('show');
    setTimeout(() => toast.remove(), 300);
  }, 2500);
}

// ============================================================
// Home Page
// ============================================================

function renderHome() {
  // Stats bar
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

  // Featured (top 4 by quality)
  const featured = [...PACKS].sort((a, b) => b.quality_score - a.quality_score).slice(0, 4);
  document.getElementById('featured-grid').innerHTML = featured.map((p, i) => renderPackCard(p, { animate: true })).join('');

  // Categories
  document.getElementById('category-grid').innerHTML = CATEGORIES.map(c => `
    <a class="category-chip" href="#/packs?domain=${c.domain}">
      <span>${c.icon}</span>
      ${c.name}
      <span class="count">${c.count}</span>
    </a>
  `).join('');

  // Trending (sorted by usage)
  const trending = [...PACKS].sort((a, b) => b.usage_count - a.usage_count).slice(0, 8);
  document.getElementById('trending-list').innerHTML = trending.map((p, i) => {
    const covStr = coverageDisplay(p.test_coverage);
    const isFull = p.test_coverage === 100 || p.test_coverage === '100%';
    return `
    <a class="trending-item" href="#/packs/${p.name}">
      <span class="trending-rank">${i + 1}</span>
      <div class="trending-info">
        <div class="trending-name">${p.name}</div>
        <div class="trending-desc">${p.description}</div>
      </div>
      <div class="trending-stats">
        <span class="stat-badge">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M20 6L9 17l-5-5"/></svg>
          <span class="test-coverage ${isFull ? 'full' : 'partial'}">${covStr}</span>
        </span>
        <span class="stat-badge">
          ${formatNumber(p.usage_count)} uses
        </span>
        ${p.zero_token ? '<span class="badge badge-zero-token" style="font-size:10px">$0</span>' : ''}
      </div>
    </a>`;
  }).join('');
}

// ============================================================
// Browse Page
// ============================================================

let currentSort = 'trending';
let currentTier = 'all';
let verifiedOnly = false;

function renderBrowse() {
  let filtered = [...PACKS];

  if (currentTier !== 'all') {
    filtered = filtered.filter(p => p.tier === currentTier);
  }
  if (verifiedOnly) {
    filtered = filtered.filter(p => p.verified);
  }

  // Check URL params for domain filter
  const hash = getHash();
  const domainMatch = hash.match(/domain=([\w-]+)/);
  if (domainMatch) {
    filtered = filtered.filter(p => p.domain === domainMatch[1]);
  }

  // Sort
  switch (currentSort) {
    case 'trending': filtered.sort((a, b) => b.usage_count - a.usage_count); break;
    case 'newest': filtered.sort((a, b) => a.version > b.version ? -1 : 1); break;
    case 'tests': filtered.sort((a, b) => {
      const aCov = typeof a.test_coverage === 'number' ? a.test_coverage : parseInt(a.test_coverage);
      const bCov = typeof b.test_coverage === 'number' ? b.test_coverage : parseInt(b.test_coverage);
      return bCov - aCov || b.functions - a.functions;
    }); break;
    case 'used': filtered.sort((a, b) => b.usage_count - a.usage_count); break;
  }

  document.getElementById('browse-grid').innerHTML = filtered.map(p => renderPackCard(p)).join('');
  document.getElementById('result-count').textContent = `${filtered.length} pack${filtered.length !== 1 ? 's' : ''} found`;

  // Domain filters
  const domains = [...new Set(PACKS.map(p => p.domain))];
  document.getElementById('domain-filters').innerHTML = domains.map(d => `
    <label class="filter-option" data-filter="domain" data-value="${d}">
      <input type="checkbox" ${domainMatch && domainMatch[1] === d ? 'checked' : ''}> ${d.charAt(0).toUpperCase() + d.slice(1)}
    </label>
  `).join('');
}

// ============================================================
// Detail Page
// ============================================================

function renderDetail(packName) {
  const pack = PACKS.find(p => p.name === packName);
  if (!pack) {
    document.getElementById('detail-header').innerHTML = `<h1>Pack not found</h1>`;
    document.getElementById('detail-body').innerHTML = `<p class="text-muted">Could not find pack "${packName}".</p>`;
    return;
  }

  const covStr = coverageDisplay(pack.test_coverage);
  const isFull = pack.test_coverage === 100 || pack.test_coverage === '100%';
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

  const fns = pack.functions_list || [];
  document.getElementById('detail-body').innerHTML = `
    <div class="detail-main">
      <!-- Description -->
      <div class="detail-section">
        <h2>About</h2>
        <p style="color:var(--color-text-muted);font-size:var(--text-sm);line-height:1.7">${pack.description}</p>
        <div style="margin-top:var(--space-4);display:flex;flex-wrap:wrap;gap:var(--space-2)">
          ${pack.tags.map(t => `<span style="padding:2px 10px;background:var(--color-surface-2);border-radius:var(--radius-full);font-size:var(--text-xs);color:var(--color-text-muted)">#${t}</span>`).join('')}
        </div>
      </div>

      <!-- Functions -->
      <div class="detail-section">
        <h2>Functions <span style="color:var(--color-text-faint);font-weight:400">(${pack.functions})</span></h2>
        ${fns.length > 0 ? `
        <div class="function-list">
          ${fns.map(fn => {
            const exStr = typeof fn.example === 'string' ? fn.example : (fn.example ? fn.example.input + ' → ' + fn.example.output : '');
            return `
            <div class="function-item">
              <div class="function-sig">${escapeHtml(fn.sig)}</div>
              <div class="function-desc">${fn.desc}</div>
              <div class="function-example">
                <span class="comment">// Example</span><br>
                <span class="output">${escapeHtml(exStr)}</span>
              </div>
            </div>
          `;}).join('')}
          ${fns.length < pack.functions ? `<p style="text-align:center;color:var(--color-text-faint);font-size:var(--text-xs);padding:var(--space-4)">+ ${pack.functions - fns.length} more functions</p>` : ''}
        </div>` : `<p style="color:var(--color-text-faint);font-size:var(--text-sm)">This pack contains ${pack.functions} deterministic functions. View the source for full signatures.</p>`}
      </div>

      <!-- Routing -->
      <div class="detail-section">
        <h2>Routing</h2>
        <p style="color:var(--color-text-muted);font-size:var(--text-sm);margin-bottom:var(--space-3)">Nanosistant automatically routes queries to this pack based on:</p>
        <div style="display:flex;flex-wrap:wrap;gap:var(--space-2);margin-bottom:var(--space-3)">
          <span style="font-size:var(--text-xs);color:var(--color-text-faint);font-weight:600;text-transform:uppercase;letter-spacing:0.06em;padding-top:3px">Keywords</span>
          ${pack.tags.map(t => `<span style="padding:3px 10px;background:var(--color-primary-subtle);border:1px solid rgba(0,200,200,0.2);border-radius:var(--radius-full);font-family:var(--font-mono);font-size:var(--text-xs);color:var(--color-primary)">${t}</span>`).join('')}
        </div>
      </div>

      <!-- Pack.toml Preview -->
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

    <!-- Sidebar -->
    <div class="detail-sidebar">
      <div class="detail-install-card">
        <h3>Quick Install</h3>
        <div class="detail-install-cmd">
          <span class="prompt">$</span>
          <code>nanosistant install ${pack.name}</code>
          <button class="copy-btn" data-copy="nanosistant install ${pack.name}" aria-label="Copy install command">
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
            <div class="value ${isFull ? 'text-green' : ''}" style="${!isFull ? 'color:var(--color-warning)' : ''}">${covStr}</div>
            <div class="label">Test Coverage</div>
          </div>
          <div class="detail-stat">
            <div class="value text-purple">${formatNumber(pack.usage_count)}</div>
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
  renderHome();
  route();

  // Mobile menu
  document.getElementById('mobile-menu-btn')?.addEventListener('click', () => {
    document.getElementById('nav-links')?.classList.toggle('open');
  });

  // Copy buttons — with toast
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
      setTimeout(() => {
        btn.classList.remove('copied');
        btn.innerHTML = origHTML;
      }, 2000);
    });
  });

  // Sort tabs
  document.querySelectorAll('.sort-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.sort-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      currentSort = tab.dataset.sort;
      renderBrowse();
    });
  });

  // Tier filter
  document.querySelectorAll('[data-filter="tier"]').forEach(opt => {
    opt.addEventListener('click', () => {
      document.querySelectorAll('[data-filter="tier"]').forEach(o => o.classList.remove('active'));
      opt.classList.add('active');
      currentTier = opt.dataset.value;
      renderBrowse();
    });
  });

  // Verified filter
  document.getElementById('verified-only')?.addEventListener('change', (e) => {
    verifiedOnly = e.target.checked;
    renderBrowse();
  });
});

window.addEventListener('hashchange', route);
