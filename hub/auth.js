// ============================================================
// NSTN Hub — Authentication + Pack Submission
// ============================================================

const API_BASE = 'https://nstn-hub-api.vercel.app';
const GH_ICON = '<svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>';

// ── Auth State ──────────────────────────────────────────────

const AUTH = {
  token: localStorage.getItem('nstn_token'),
  user: JSON.parse(localStorage.getItem('nstn_user') || 'null'),

  isLoggedIn() { return !!this.token && !!this.user; },

  login() { window.location.href = `${API_BASE}/api/auth/login`; },

  async handleCallback(code) {
    try {
      const res = await fetch(`${API_BASE}/api/auth/callback`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code }),
      });
      const data = await res.json();
      if (data.access_token) {
        this.token = data.access_token;
        this.user = data.user;
        localStorage.setItem('nstn_token', data.access_token);
        localStorage.setItem('nstn_user', JSON.stringify(data.user));
      }
      return data;
    } catch (e) { console.error('Auth callback failed:', e); return null; }
  },

  logout() {
    this.token = null;
    this.user = null;
    localStorage.removeItem('nstn_token');
    localStorage.removeItem('nstn_user');
    updateAuthUI();
  },
};

// Check for OAuth callback on load
(function checkAuthCallback() {
  const params = new URLSearchParams(window.location.search);
  const code = params.get('code');
  if (code) {
    AUTH.handleCallback(code).then(() => {
      window.history.replaceState({}, '', window.location.pathname + (window.location.hash || ''));
      updateAuthUI();
    });
  }
})();

// ── Auth UI ─────────────────────────────────────────────────

function updateAuthUI() {
  const el = document.getElementById('nav-auth');
  if (!el) return;
  if (AUTH.isLoggedIn()) {
    el.innerHTML = `
      <div class="user-menu">
        <img src="${AUTH.user.avatar_url}" alt="" class="avatar" />
        <span class="user-name">${AUTH.user.login}</span>
        <button onclick="AUTH.logout()" class="btn-signout">Sign out</button>
      </div>`;
  } else {
    el.innerHTML = `<button onclick="AUTH.login()" class="btn-signin">${GH_ICON} Sign in with GitHub</button>`;
  }
}

// Init auth UI once DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  updateAuthUI();
  updateSubmitAuthStatus();
});

function updateSubmitAuthStatus() {
  const el = document.getElementById('submit-auth-status');
  if (!el) return;
  if (AUTH.isLoggedIn()) {
    el.innerHTML = `<div class="user-menu"><img src="${AUTH.user.avatar_url}" class="avatar" /><span class="user-name">@${AUTH.user.login}</span><button onclick="AUTH.logout()" class="btn-signout">Sign out</button></div>`;
    // Enable submit button
    const btn = document.getElementById('submit-btn');
    if (btn) { btn.disabled = false; btn.textContent = 'Submit Pack'; }
  } else {
    el.innerHTML = `<button onclick="AUTH.login()" class="btn-signin">${GH_ICON} Sign in with GitHub to submit</button>`;
    const btn = document.getElementById('submit-btn');
    if (btn) { btn.disabled = true; btn.textContent = 'Sign in to submit'; }
  }
}

// ── Form State ──────────────────────────────────────────────

let formRules = [makeEmptyRule()];
let activeTab = 'toml';

function makeEmptyRule() {
  return {
    id: 'rule-' + Date.now(),
    description: '',
    keywords: [],
    formulaType: 'Static',
    formulaData: { response: '' },
    confidence: 0.8,
    exampleInput: '',
    exampleOutput: '',
  };
}

// ── Render Submit Page ──────────────────────────────────────

function renderSubmitPage() {
  return `
  <div class="submit-page">
    <h1>Submit a Pack</h1>
    <p class="submit-subtitle">Create and submit a deterministic function pack directly from the browser. Your submission becomes a PR to the nanosistant repo.</p>

    ${!AUTH.isLoggedIn() ? `<div class="auth-banner"><span>Sign in with GitHub to submit packs</span><button onclick="AUTH.login()" class="btn-signin">${GH_ICON} Sign in</button></div>` : ''}

    <div class="form-grid">
      <div class="form-main">
        <!-- Pack Metadata -->
        <div class="form-section">
          <h2>Pack Metadata</h2>
          <label class="form-label">Pack Name <span class="required">*</span></label>
          <input type="text" id="f-name" class="form-input" placeholder="my-awesome-pack" oninput="onFormChange()" pattern="[a-z0-9-]+" />
          <small class="form-hint">Lowercase, hyphens only. e.g. jersey-club-production</small>

          <label class="form-label">Description <span class="required">*</span></label>
          <textarea id="f-desc" class="form-input form-textarea" placeholder="What does this pack do?" oninput="onFormChange()"></textarea>

          <div class="form-row">
            <div class="form-col">
              <label class="form-label">Domain <span class="required">*</span></label>
              <select id="f-domain" class="form-input form-select" onchange="onFormChange()">
                <option value="music">Music</option><option value="finance">Finance</option>
                <option value="data">Data</option><option value="time">Time</option>
                <option value="text">Text</option><option value="code">Code</option>
                <option value="geo">Geo</option><option value="physics">Physics</option>
                <option value="health">Health</option><option value="social">Social</option>
                <option value="universal">Universal</option><option value="custom">Custom</option>
              </select>
            </div>
            <div class="form-col">
              <label class="form-label">Tier <span class="required">*</span></label>
              <div class="radio-group">
                <label><input type="radio" name="tier" value="Domain" checked onchange="onFormChange()" /> Domain</label>
                <label><input type="radio" name="tier" value="Universal" onchange="onFormChange()" /> Universal</label>
                <label><input type="radio" name="tier" value="Operator" onchange="onFormChange()" /> Operator</label>
              </div>
            </div>
          </div>

          <label class="form-label">Tags</label>
          <input type="text" id="f-tags" class="form-input" placeholder="tag1, tag2, tag3" oninput="onFormChange()" />

          <div class="form-row">
            <div class="form-col">
              <label class="form-label">Version</label>
              <input type="text" id="f-version" class="form-input" value="0.1.0" oninput="onFormChange()" />
            </div>
            <div class="form-col">
              <label class="form-label">License</label>
              <input type="text" id="f-license" class="form-input" value="MIT" oninput="onFormChange()" />
            </div>
          </div>
        </div>

        <!-- Function Type Tabs -->
        <div class="form-section">
          <h2>Functions</h2>
          <div class="tab-bar">
            <button class="tab ${activeTab === 'toml' ? 'active' : ''}" onclick="switchTab('toml')">TOML Rules</button>
            <button class="tab ${activeTab === 'rust' ? 'active' : ''}" onclick="switchTab('rust')">Rust Code</button>
          </div>

          <div id="tab-toml" style="display:${activeTab === 'toml' ? 'block' : 'none'}">
            <div id="rules-container">${formRules.map((r, i) => renderRuleForm(r, i)).join('')}</div>
            <button class="btn-add-rule" onclick="addRule()">+ Add another rule</button>
          </div>

          <div id="tab-rust" style="display:${activeTab === 'rust' ? 'block' : 'none'}">
            <label class="form-label">functions.rs</label>
            <textarea id="f-rust" class="form-input code-editor" placeholder="pub fn my_function(x: f64) -> f64 {\n    // Pure deterministic function\n    x * 2.0\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn test_my_function() {\n        assert_eq!(my_function(5.0), 10.0);\n    }\n}" oninput="onFormChange()"></textarea>
          </div>
        </div>

        <!-- Actions -->
        <div class="form-section form-actions">
          <button class="btn-validate" onclick="validatePack()">Validate</button>
          <button class="btn-submit ${AUTH.isLoggedIn() ? '' : 'disabled'}" onclick="submitPack()" ${AUTH.isLoggedIn() ? '' : 'disabled'}>
            ${AUTH.isLoggedIn() ? 'Submit Pack' : 'Sign in to submit'}
          </button>
          <div id="validation-results"></div>
          <div id="submit-results"></div>
        </div>
      </div>

      <!-- Live Preview -->
      <div class="form-preview">
        <h3>Live Preview</h3>
        <div class="preview-label">pack.toml</div>
        <pre class="preview-panel" id="preview-pack"></pre>
        <div class="preview-label" style="margin-top:1rem">rules.toml</div>
        <pre class="preview-panel" id="preview-rules"></pre>
      </div>
    </div>
  </div>`;
}

function renderRuleForm(rule, index) {
  return `
  <div class="rule-card" data-index="${index}">
    <div class="rule-header">
      <span>Rule ${index + 1}</span>
      ${index > 0 ? `<button class="btn-remove" onclick="removeRule(${index})">Remove</button>` : ''}
    </div>

    <label class="form-label">Description</label>
    <input type="text" class="form-input rule-desc" value="${esc(rule.description)}" oninput="updateRule(${index},'description',this.value)" placeholder="What does this rule do?" />

    <label class="form-label">Trigger Keywords</label>
    <input type="text" class="form-input rule-kw" value="${rule.keywords.join(', ')}" oninput="updateRuleKeywords(${index},this.value)" placeholder="keyword1, keyword2" />

    <label class="form-label">Formula Type</label>
    <select class="form-input form-select rule-ftype" onchange="updateRuleFormula(${index},this.value)">
      <option ${rule.formulaType==='Static'?'selected':''}>Static</option>
      <option ${rule.formulaType==='Arithmetic'?'selected':''}>Arithmetic</option>
      <option ${rule.formulaType==='Lookup'?'selected':''}>Lookup</option>
      <option ${rule.formulaType==='WeightedScore'?'selected':''}>WeightedScore</option>
      <option ${rule.formulaType==='Template'?'selected':''}>Template</option>
    </select>

    <div class="formula-fields">${renderFormulaFields(rule, index)}</div>

    <div class="form-row">
      <div class="form-col">
        <label class="form-label">Example Input</label>
        <input type="text" class="form-input" value="${esc(rule.exampleInput)}" oninput="updateRule(${index},'exampleInput',this.value)" />
      </div>
      <div class="form-col">
        <label class="form-label">Expected Output</label>
        <input type="text" class="form-input" value="${esc(rule.exampleOutput)}" oninput="updateRule(${index},'exampleOutput',this.value)" />
      </div>
    </div>

    <label class="form-label">Confidence: ${rule.confidence}</label>
    <input type="range" min="0.5" max="1.0" step="0.05" value="${rule.confidence}" oninput="updateRule(${index},'confidence',parseFloat(this.value));this.previousElementSibling.textContent='Confidence: '+this.value" />
  </div>`;
}

function renderFormulaFields(rule, index) {
  switch(rule.formulaType) {
    case 'Static':
      return `<label class="form-label">Response</label><textarea class="form-input form-textarea" oninput="updateRuleFormulaData(${index},{response:this.value})" placeholder="The deterministic response">${esc(rule.formulaData.response||'')}</textarea>`;
    case 'Arithmetic':
      return `<label class="form-label">Expression</label><input class="form-input" value="${esc(rule.formulaData.expr||'')}" oninput="updateRuleFormulaData(${index},{expr:this.value})" placeholder="x * 2 + 3" /><label class="form-label">Variables (comma separated)</label><input class="form-input" value="${esc((rule.formulaData.variables||[]).join(', '))}" oninput="updateRuleFormulaData(${index},{variables:this.value.split(',').map(s=>s.trim())})" placeholder="x, y" />`;
    case 'Lookup':
      return `<label class="form-label">Lookup Table (one per line: key = value)</label><textarea class="form-input form-textarea" oninput="parseLookup(${index},this.value)" placeholder="monday = 1\ntuesday = 2\nwednesday = 3">${(rule.formulaData.pairs||[]).map(([k,v])=>k+' = '+v).join('\n')}</textarea>`;
    case 'WeightedScore':
      return `<label class="form-label">Keyword Weights (one per line: keyword = weight)</label><textarea class="form-input form-textarea" oninput="parseWeights(${index},this.value)" placeholder="verse = 1.0\nhook = 0.9\nbeat = 0.7">${(rule.formulaData.weights||[]).map(([k,w])=>k+' = '+w).join('\n')}</textarea>`;
    case 'Template':
      return `<label class="form-label">Template</label><input class="form-input" value="${esc(rule.formulaData.template||'')}" oninput="updateRuleFormulaData(${index},{template:this.value})" placeholder="At {bpm} BPM: {result}" /><label class="form-label">Slots (comma separated)</label><input class="form-input" value="${esc((rule.formulaData.slots||[]).join(', '))}" oninput="updateRuleFormulaData(${index},{slots:this.value.split(',').map(s=>s.trim())})" placeholder="bpm, result" />`;
    default: return '';
  }
}

// ── Form Helpers ────────────────────────────────────────────

function switchTab(tab) {
  activeTab = tab;
  document.getElementById('tab-toml').style.display = tab === 'toml' ? 'block' : 'none';
  document.getElementById('tab-rust').style.display = tab === 'rust' ? 'block' : 'none';
  document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.textContent.toLowerCase().includes(tab)));
  onFormChange();
}

function addRule() {
  formRules.push(makeEmptyRule());
  document.getElementById('rules-container').innerHTML = formRules.map((r,i) => renderRuleForm(r,i)).join('');
  onFormChange();
}

function removeRule(i) {
  formRules.splice(i, 1);
  document.getElementById('rules-container').innerHTML = formRules.map((r,i) => renderRuleForm(r,i)).join('');
  onFormChange();
}

function updateRule(i, key, val) { formRules[i][key] = val; onFormChange(); }
function updateRuleKeywords(i, val) { formRules[i].keywords = val.split(',').map(s=>s.trim()).filter(Boolean); onFormChange(); }
function updateRuleFormula(i, type) { formRules[i].formulaType = type; formRules[i].formulaData = {}; document.getElementById('rules-container').innerHTML = formRules.map((r,j)=>renderRuleForm(r,j)).join(''); onFormChange(); }
function updateRuleFormulaData(i, obj) { Object.assign(formRules[i].formulaData, obj); onFormChange(); }
function parseLookup(i, text) { formRules[i].formulaData.pairs = text.split('\n').filter(l=>l.includes('=')).map(l=>{const[k,...v]=l.split('=');return[k.trim(),v.join('=').trim()];}); onFormChange(); }
function parseWeights(i, text) { formRules[i].formulaData.weights = text.split('\n').filter(l=>l.includes('=')).map(l=>{const[k,w]=l.split('=');return[k.trim(),parseFloat(w)||0];}); onFormChange(); }
function esc(s) { return (s||'').replace(/"/g,'&quot;').replace(/</g,'&lt;'); }

// ── TOML Generator ──────────────────────────────────────────

function generatePackToml() {
  const name = (document.getElementById('f-name')?.value || 'my-pack').trim();
  const desc = (document.getElementById('f-desc')?.value || '').trim();
  const domain = document.getElementById('f-domain')?.value || 'custom';
  const tier = document.querySelector('input[name="tier"]:checked')?.value || 'Domain';
  const tags = (document.getElementById('f-tags')?.value || '').split(',').map(s=>s.trim()).filter(Boolean);
  const version = document.getElementById('f-version')?.value || '0.1.0';
  const license = document.getElementById('f-license')?.value || 'MIT';
  const author = AUTH.isLoggedIn() ? AUTH.user.login : 'anonymous';

  return `[pack]
name = "${name}"
version = "${version}"
author = "${author}"
description = "${desc}"
domain = "${domain}"
tier = "${tier}"
license = "${license}"
nstn_version = ">=0.7.0"
tags = [${tags.map(t=>`"${t}"`).join(', ')}]
functions = ${formRules.filter(r=>r.description).length}

[pack.routing]
keywords = [${formRules.flatMap(r=>r.keywords).map(k=>`"${k}"`).join(', ')}]
confidence_threshold = 0.6`;
}

function generateRulesToml() {
  const name = (document.getElementById('f-name')?.value || 'my-pack').trim();
  let toml = `[meta]\nversion = "0.1.0"\npack_name = "${name}"\napproved = false\n\n`;
  for (const r of formRules) {
    if (!r.description) continue;
    toml += `[[rules]]\nid = "${r.id}"\ndescription = "${r.description}"\n`;
    toml += `trigger_keywords = [${r.keywords.map(k=>`"${k}"`).join(', ')}]\n`;
    toml += `confidence = ${r.confidence}\n`;
    switch(r.formulaType) {
      case 'Static':
        toml += `[rules.formula]\ntype = "Static"\nresponse = "${(r.formulaData.response||'').replace(/"/g,'\\"')}"\n`; break;
      case 'Arithmetic':
        toml += `[rules.formula]\ntype = "Arithmetic"\nexpr = "${r.formulaData.expr||''}"\nvariables = [${(r.formulaData.variables||[]).map(v=>`"${v}"`).join(', ')}]\n`; break;
      case 'Lookup':
        toml += `[rules.formula]\ntype = "Lookup"\n[rules.formula.table]\n`;
        for (const [k,v] of (r.formulaData.pairs||[])) toml += `${k} = "${v}"\n`; break;
      case 'WeightedScore':
        toml += `[rules.formula]\ntype = "WeightedScore"\n[rules.formula.weights]\n`;
        for (const [k,w] of (r.formulaData.weights||[])) toml += `${k} = ${w}\n`; break;
      case 'Template':
        toml += `[rules.formula]\ntype = "Template"\ntemplate = "${r.formulaData.template||''}"\nslots = [${(r.formulaData.slots||[]).map(s=>`"${s}"`).join(', ')}]\n`; break;
    }
    if (r.exampleInput || r.exampleOutput) {
      toml += `\n[[rules.examples]]\ninput = "${r.exampleInput}"\nexpected_output = "${r.exampleOutput}"\n`;
    }
    toml += '\n';
  }
  return toml;
}

function onFormChange() {
  const packEl = document.getElementById('preview-pack');
  const rulesEl = document.getElementById('preview-rules');
  if (packEl) packEl.textContent = generatePackToml();
  if (rulesEl) rulesEl.textContent = generateRulesToml();
}

// ── Validate ────────────────────────────────────────────────

async function validatePack() {
  const el = document.getElementById('validation-results');
  el.innerHTML = '<span class="validation-info">Validating...</span>';
  try {
    const res = await fetch(`${API_BASE}/api/validate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        pack_toml: generatePackToml(),
        rules_toml: activeTab === 'toml' ? generateRulesToml() : null,
        functions_rs: activeTab === 'rust' ? document.getElementById('f-rust')?.value : null,
      }),
    });
    const data = await res.json();
    if (data.valid) {
      el.innerHTML = '<span class="validation-success">✓ Pack is valid</span>' +
        data.warnings.map(w=>`<div class="validation-warning">⚠ ${w}</div>`).join('');
    } else {
      el.innerHTML = data.errors.map(e=>`<div class="validation-error">✗ ${e}</div>`).join('') +
        data.warnings.map(w=>`<div class="validation-warning">⚠ ${w}</div>`).join('');
    }
  } catch(e) { el.innerHTML = `<div class="validation-error">✗ Validation failed: ${e.message}</div>`; }
}

// ── Submit ──────────────────────────────────────────────────

async function submitPack() {
  if (!AUTH.isLoggedIn()) return AUTH.login();
  const el = document.getElementById('submit-results');
  const name = document.getElementById('f-name')?.value?.trim();
  if (!name) { el.innerHTML = '<div class="validation-error">Pack name is required</div>'; return; }

  el.innerHTML = '<span class="validation-info">Submitting... (forking repo, creating PR)</span>';
  try {
    const res = await fetch(`${API_BASE}/api/submit`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${AUTH.token}` },
      body: JSON.stringify({
        pack_name: name,
        pack_toml: generatePackToml(),
        rules_toml: activeTab === 'toml' ? generateRulesToml() : null,
        functions_rs: activeTab === 'rust' ? document.getElementById('f-rust')?.value : null,
      }),
    });
    const data = await res.json();
    if (data.success) {
      el.innerHTML = `<div class="submit-success"><h3>Pack submitted successfully!</h3><p>PR created: <a href="${data.pr_url}" target="_blank">#${data.pr_number}</a></p><p>We'll review and merge your pack. Thanks for contributing!</p></div>`;
    } else {
      el.innerHTML = `<div class="validation-error">✗ ${data.error}${data.details ? ': ' + data.details : ''}</div>`;
    }
  } catch(e) { el.innerHTML = `<div class="validation-error">✗ Submission failed: ${e.message}</div>`; }
}
