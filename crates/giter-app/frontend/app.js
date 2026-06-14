/* ── Tauri IPC ──────────────────────────────────────────── */
const { invoke } = window.__TAURI__?.core ?? {};

async function cmd(name, args = {}) {
  if (!invoke) throw new Error('Tauri IPC not available (running outside webview?)');
  return await invoke(name, args);
}

// ── State ─────────────────────────────────────────────────
let repos = [];          // string[]
let scanResults = new Map(); // path -> { state, data? }
let selectedPath = null;
let settings = null;

// ── DOM refs ──────────────────────────────────────────────
const $ = (id) => document.getElementById(id);
const repoListEl = $('repo-list');
const statusBar = $('status-bar');
const welcomeEl = $('welcome');
const statsPanel = $('stats-panel');
const statsRepoName = $('stats-repo-name');
const statsState = $('stats-state');
const statCommits = $('stat-commits');
const statFiles = $('stat-files');
const statInsertions = $('stat-insertions');
const statDeletions = $('stat-deletions');
const commitsBody = $('commits-body');

// ── Initialization ────────────────────────────────────────
document.addEventListener('DOMContentLoaded', async () => {
  $('btn-open').addEventListener('click', openRepo);
  $('btn-scan-all').addEventListener('click', scanAll);
  $('btn-scan-selected').addEventListener('click', scanSelected);

  // Tab switching
  document.getElementById('tab-bar')?.addEventListener('click', (e) => {
    const btn = e.target.closest('.tab');
    if (!btn) return;
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');
    document.querySelectorAll('.tab-panel').forEach(p => p.classList.remove('active'));
    document.getElementById('panel-' + btn.dataset.panel)?.classList.add('active');
  });

  // Trend mode switching
  document.getElementById('trend-mode-bar')?.addEventListener('click', (e) => {
    const btn = e.target.closest('.trend-btn');
    if (!btn) return;
    renderTrendMode(btn.dataset.mode);
  });

  await loadInitialState();
});

async function loadInitialState() {
  try {
    settings = await cmd('get_settings');
    repos = await cmd('get_repos');
    const results = await cmd('get_scan_results');
    results.forEach(([path, state]) => scanResults.set(path, state));
    renderRepoList();
    if (repos.length > 0) {
      selectRepo(repos[0]);
    }
    setStatus('就绪');
  } catch (e) {
    setStatus('加载状态失败：' + e, true);
  }
}

// ── Repo management ──────────────────────────────────────
async function openRepo() {
  try {
    const path = await cmd('pick_folder');
    if (!path) return;
    setStatus('正在添加仓库…');
    await cmd('add_repo', { path });
    repos = await cmd('get_repos');
    renderRepoList();
    selectRepo(path);
    setStatus('已添加：' + path);
  } catch (e) {
    setStatus('错误：' + e, true);
  }
}

async function removeRepo(path) {
  try {
    await cmd('remove_repo', { path });
    repos = await cmd('get_repos');
    scanResults.delete(path);
    if (selectedPath === path) {
      selectedPath = repos.length > 0 ? repos[0] : null;
    }
    renderRepoList();
    if (selectedPath) showStats(selectedPath);
    else { welcomeEl.style.display = ''; statsPanel.style.display = 'none'; }
  } catch (e) {
    setStatus('错误：' + e, true);
  }
}

function selectRepo(path) {
  selectedPath = path;
  renderRepoList();
  showStats(path);
}

// ── Scanning ─────────────────────────────────────────────
async function scanRepo(path) {
  // Optimistic UI: show scanning state immediately
  scanResults.set(path, { state: 'Scanning' });
  renderRepoList();
  if (selectedPath === path) showStats(path);
  setStatus('正在扫描：' + path);

  try {
    await cmd('scan_repo', { path });
    const results = await cmd('get_scan_results');
    scanResults.set(path, results.find(([p]) => p === path)?.[1] ?? { state: 'Failed', data: null, msg: '未知错误' });
  } catch (e) {
    scanResults.set(path, { state: 'Failed', data: null, msg: String(e) });
  }
  renderRepoList();
  if (selectedPath === path) showStats(path);
  setStatus('扫描完成：' + path);
}

async function scanAll() {
  for (const path of repos) {
    await scanRepo(path);
  }
}

async function scanSelected() {
  if (selectedPath) await scanRepo(selectedPath);
}

// ── Rendering ────────────────────────────────────────────
function renderRepoList() {
  if (repos.length === 0) {
    repoListEl.innerHTML = '<p class="hint">尚未添加仓库。<br/>点击「打开仓库」开始。</p>';
    return;
  }
  repoListEl.innerHTML = repos.map(path => {
    const name = path.split(/[/\\]/).pop() || path;
    const result = scanResults.get(path) ?? { state: 'NotScanned' };
    let stateClass = 'not-scanned';
    if (typeof result === 'object' && result.state) {
      stateClass = result.state.toLowerCase();
    } else if (typeof result === 'string') {
      stateClass = result.toLowerCase();
    }
    const selected = path === selectedPath ? 'selected' : '';
    return `
      <div class="repo-item ${selected}" data-path="${escapeHtml(path)}">
        <span class="status-dot ${stateClass}"></span>
        <span class="repo-name">${escapeHtml(name)}</span>
        <button class="remove-btn" data-path="${escapeHtml(path)}">&times;</button>
      </div>
    `;
  }).join('');

  // Attach event listeners
  repoListEl.querySelectorAll('.repo-item').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e.target.classList.contains('remove-btn')) return;
      selectRepo(el.dataset.path);
    });
  });
  repoListEl.querySelectorAll('.remove-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      removeRepo(btn.dataset.path);
    });
  });
}

function translateState(state) {
  const map = { 'NotScanned': '未扫描', 'Scanning': '扫描中', 'Completed': '已完成', 'Failed': '失败' };
  return map[state] ?? state;
}

function showStats(path) {
  const result = scanResults.get(path) ?? { state: 'NotScanned' };
  const name = path.split(/[/\\]/).pop() || path;
  welcomeEl.style.display = 'none';
  statsPanel.style.display = '';
  statsRepoName.textContent = name;

  let stateStr = '未扫描';
  let stateClass = 'not-scanned';
  let stats = null;

  if (typeof result === 'string') {
    stateStr = translateState(result);
    if (result === 'Completed') { stateClass = 'completed'; stats = null; }
    else if (result === 'Scanning') stateClass = 'scanning';
    else if (result === 'Failed') stateClass = 'failed';
  } else if (result?.state) {
    stateStr = translateState(result.state);
    stateClass = result.state.toLowerCase();
    if (result.state === 'Completed' && result.data) {
      stats = result.data;
    }
  } else if (result?.commits) {
    // Direct RepoStats object
    stateStr = '已完成';
    stateClass = 'completed';
    stats = result;
  }

  statsState.textContent = stateStr;
  statsState.className = 'badge ' + stateClass;

  if (stats) {
    statCommits.textContent = stats.commits?.length ?? '—';
    statFiles.textContent = stats.changes?.length ?? '—';
    statInsertions.textContent = stats.changes?.reduce((s, c) => s + (c.insertions ?? 0), 0) ?? '—';
    statDeletions.textContent = stats.changes?.reduce((s, c) => s + (c.deletions ?? 0), 0) ?? '—';
    renderCommits(stats.commits);
    renderChanges(stats.changes);
    renderFileTypes(stats.changes);
    loadContributors(path);
    loadTrends(path);
  } else {
    statCommits.textContent = '—';
    statFiles.textContent = '—';
    statInsertions.textContent = '—';
    statDeletions.textContent = '—';
    commitsBody.innerHTML = '<tr><td colspan="4" style="text-align:center;color:#999;">暂无数据</td></tr>';
    // Clear all extra panels
    document.querySelectorAll('#changes-body, #contributors-list, #languages-chart, #trend-chart')
      .forEach(el => { if (el) el.innerHTML = ''; });
  }
}

function renderCommits(commits) {
  if (!commits || commits.length === 0) {
    commitsBody.innerHTML = '<tr><td colspan="4" style="text-align:center;color:#999;">未找到提交</td></tr>';
    return;
  }
  commitsBody.innerHTML = commits.map(c => `
    <tr>
      <td class="hash">${escapeHtml(c.hash?.slice(0, 8) ?? '')}</td>
      <td>${escapeHtml(c.author_name ?? '')}</td>
      <td>${escapeHtml(formatDate(c.datetime))}</td>
      <td>${escapeHtml(truncate(c.message, 60))}</td>
    </tr>
  `).join('');
}

// ── File changes detail ──────────────────────────────────
function renderChanges(changes) {
  const body = $('changes-body');
  if (!changes || changes.length === 0) {
    body.innerHTML = '<tr><td colspan="5" style="text-align:center;color:#999;">暂无文件变更</td></tr>';
    return;
  }
  body.innerHTML = changes.map(c => `
    <tr>
      <td class="file-path" title="${escapeHtml(c.file_path ?? '')}">${escapeHtml(c.file_path ?? '')}</td>
      <td><span class="ext-badge">${escapeHtml(c.ext ?? '')}</span></td>
      <td style="color:#43a047;">+${c.insertions ?? 0}</td>
      <td style="color:#e53935;">-${c.deletions ?? 0}</td>
      <td class="hash">${escapeHtml((c.commit_hash ?? '').slice(0, 8))}</td>
    </tr>
  `).join('');
}

// ── Contributor statistics ───────────────────────────────
async function loadContributors(path) {
  const list = $('contributors-list');
  try {
    const data = await cmd('get_contributors', { path });
    renderContributors(list, data);
  } catch (e) {
    list.innerHTML = '<div class="contributor-empty">加载失败：' + escapeHtml(String(e)) + '</div>';
  }
}

function renderContributors(container, data) {
  if (!data || data.length === 0) {
    container.innerHTML = '<div class="contributor-empty">暂无贡献者数据</div>';
    return;
  }
  container.innerHTML = data.map(([name, count]) => `
    <div class="contributor-row">
      <span class="contributor-name">${escapeHtml(name)}</span>
      <span class="contributor-count">${count} 次提交</span>
    </div>
  `).join('');
}

// ── File type distribution (computed on frontend) ────────
function renderFileTypes(changes) {
  const container = $('languages-chart');
  if (!changes || changes.length === 0) {
    container.innerHTML = '<div class="trend-empty">暂无文件类型数据</div>';
    return;
  }

  // Count by extension
  const counts = {};
  for (const c of changes) {
    const ext = (c.ext || '(无)').toLowerCase();
    counts[ext] = (counts[ext] || 0) + 1;
  }
  const entries = Object.entries(counts).sort((a, b) => b[1] - a[1]);
  const maxCount = entries[0][1];

  container.innerHTML = entries.map(([ext, count]) => {
    const pct = (count / maxCount) * 100;
    return `
      <div class="bar-row">
        <span class="bar-label">${escapeHtml(ext)}</span>
        <div class="bar-track"><div class="bar-fill" style="width:${pct}%"></div></div>
        <span class="bar-count">${count}</span>
      </div>
    `;
  }).join('');
}

// ── Trends chart (commits over time + code churn) ────────
let _trendData = null;

async function loadTrends(path) {
  const chart = $('trend-chart');
  try {
    const [byDay, byWeek, byMonth, churn] = await cmd('get_trends', { path });
    _trendData = { byDay, byWeek, byMonth, churn };
    // Default to "day" view
    renderTrendMode('day');
  } catch (e) {
    chart.innerHTML = '<div class="trend-empty">加载趋势失败：' + escapeHtml(String(e)) + '</div>';
  }
}

function renderTrendMode(mode) {
  const chart = $('trend-chart');
  if (!_trendData) {
    chart.innerHTML = '<div class="trend-empty">暂无趋势数据</div>';
    return;
  }

  let data;
  let label;
  let isChurn = false;
  if (mode === 'day') { data = _trendData.byDay; label = '天'; }
  else if (mode === 'week') { data = _trendData.byWeek; label = '周'; }
  else if (mode === 'month') { data = _trendData.byMonth; label = '月'; }
  else if (mode === 'churn') { data = _trendData.churn; label = ''; isChurn = true; }

  if (!data || data.length === 0) {
    chart.innerHTML = '<div class="trend-empty">暂无趋势数据</div>';
    return;
  }

  // Update active trend button
  document.querySelectorAll('.trend-btn').forEach(b => {
    b.classList.toggle('active', b.dataset.mode === mode);
  });

  if (isChurn) {
    // Code churn: side-by-side bars (green ins, red del)
    const maxVal = Math.max(...data.flatMap(([, ins, del]) => [ins, del]));
    const scale = maxVal > 0 ? 140 / maxVal : 1;

    chart.innerHTML = `
      <div style="margin-bottom:8px;font-size:13px;color:#555;">
        <span style="display:inline-block;width:12px;height:12px;background:#43a047;border-radius:2px;vertical-align:middle;margin-right:4px;"></span> 新增
        <span style="display:inline-block;width:12px;height:12px;background:#e53935;border-radius:2px;vertical-align:middle;margin-right:4px;margin-left:16px;"></span> 删除
      </div>
      <div class="trend-bars">
        ${data.map(([date, ins, del]) => `
          <div style="flex:1;min-width:24px;display:flex;flex-direction:column;align-items:center;gap:2px;">
            <div style="width:100%;display:flex;gap:3px;align-items:flex-end;flex:1;justify-content:center;">
              <div class="trend-bar ins" style="height:${Math.max(ins * scale, 1)}px;max-width:20px;" title="${escapeHtml(date)}: +${ins}"></div>
              <div class="trend-bar del" style="height:${Math.max(del * scale, 1)}px;max-width:20px;" title="${escapeHtml(date)}: -${del}"></div>
            </div>
          </div>
        `).join('')}
      </div>
      <div class="trend-labels">
        ${data.map(([date]) => `
          <span class="trend-label" style="min-width:24px;">${escapeHtml(date.slice(5))}</span>
        `).join('')}
      </div>
    `;
  } else {
    // Commit count bar chart
    const maxCount = Math.max(...data.map(([, c]) => c), 1);
    const scale = maxCount > 0 ? 140 / maxCount : 1;

    chart.innerHTML = `
      <div class="trend-bars">
        ${data.map(([date, count]) => `
          <div class="trend-bar" style="height:${Math.max(count * scale, 1)}px" title="${escapeHtml(date)}: ${count} 次提交"></div>
        `).join('')}
      </div>
      <div class="trend-labels">
        ${data.map(([date]) => `
          <span class="trend-label">${escapeHtml(date.slice(5))}</span>
        `).join('')}
      </div>
    `;
  }
}

// ── Utilities ────────────────────────────────────────────
function setStatus(msg, isError = false) {
  statusBar.textContent = msg;
  statusBar.style.color = isError ? '#ff6b6b' : '';
}

function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function truncate(s, max) {
  if (!s) return '';
  return s.length > max ? s.slice(0, max) + '…' : s;
}

function formatDate(d) {
  if (!d) return '';
  if (typeof d === 'string') return d.slice(0, 10);
  return String(d);
}
