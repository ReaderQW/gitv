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

// Sort state
let commitSort = { key: null, asc: true };
let changeSort = { key: null, asc: true };

// Filter state
let commitFilter = '';

// Chart mode
let langChartMode = 'bar'; // 'bar' | 'pie'
let trendChartMode = 'bar'; // 'bar' | 'line' | 'curve'

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

  // Trend chart type switching
  document.getElementById('trend-chart-type-bar')?.addEventListener('click', (e) => {
    const btn = e.target.closest('.chart-type-btn');
    if (!btn || !btn.dataset.trendType) return;
    document.querySelectorAll('#trend-chart-type-bar .chart-type-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    trendChartMode = btn.dataset.trendType;
    // Re-render current trend mode
    const activeMode = document.querySelector('.trend-btn.active')?.dataset.mode || 'day';
    renderTrendMode(activeMode);
  });

  // Language chart toggle
  $('btn-toggle-lang')?.addEventListener('click', () => {
    langChartMode = langChartMode === 'bar' ? 'pie' : 'bar';
    $('btn-toggle-lang').textContent = langChartMode === 'pie' ? '切换为柱状图' : '切换为饼图';
    if (selectedPath) showStats(selectedPath);
  });

  // Table header sorting (delegated)
  document.querySelector('#panel-commits thead')?.addEventListener('click', (e) => {
    const th = e.target.closest('.sortable');
    if (!th) return;
    handleSort(th.dataset.key, true);
  });
  document.querySelector('#panel-changes thead')?.addEventListener('click', (e) => {
    const th = e.target.closest('.sortable');
    if (!th) return;
    handleSort(th.dataset.key, false);
  });

  // Export buttons
  $('btn-export-csv')?.addEventListener('click', () => doExport('csv'));
  $('btn-export-json')?.addEventListener('click', () => doExport('json'));

  // Commit search filter
  $('commit-search')?.addEventListener('input', (e) => {
    commitFilter = e.target.value;
    const result = scanResults.get(selectedPath);
    if (result?.state === 'Completed' && result.data?.commits) {
      renderCommits(result.data.commits);
    }
  });

  // Settings
  $('btn-settings')?.addEventListener('click', () => {
    $('settings-max-recent').value = settings?.max_recent ?? 10;
    $('settings-max-commits').value = settings?.max_commits ?? 5000;
    $('settings-repo-count').textContent = settings?.recent_repos?.length ?? 0;
    $('settings-overlay').style.display = 'flex';
  });
  $('btn-settings-cancel')?.addEventListener('click', () => {
    $('settings-overlay').style.display = 'none';
  });
  $('btn-settings-save')?.addEventListener('click', saveSettings);
  $('settings-overlay')?.addEventListener('click', (e) => {
    if (e.target === $('settings-overlay')) {
      $('settings-overlay').style.display = 'none';
    }
  });

  // Listen for scan progress events from backend
  if (window.__TAURI__?.event) {
    window.__TAURI__.event.listen('scan-progress', (event) => {
      const { path, current, total } = event.payload;
      // Update status bar with progress
      setStatus(`扫描中: ${current}/${total} 个提交`);

      // Update badge if this is the currently selected repo
      if (path === selectedPath) {
        statsState.textContent = `扫描中 ${current}/${total}`;
      }
    });
  }

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

// ── Sorting ────────────────────────────────────────────────
function handleSort(key, isCommits) {
  const sort = isCommits ? commitSort : changeSort;
  if (sort.key === key) sort.asc = !sort.asc;
  else { sort.key = key; sort.asc = true; }
  // Re-render with current data
  if (selectedPath) showStats(selectedPath);
}

function sortData(data, sort) {
  if (!sort.key || !data) return data;
  return [...data].sort((a, b) => {
    const va = a[sort.key];
    const vb = b[sort.key];
    if (typeof va === 'number' && typeof vb === 'number') {
      return sort.asc ? va - vb : vb - va;
    }
    const sa = String(va ?? '').toLowerCase();
    const sb = String(vb ?? '').toLowerCase();
    return sort.asc ? sa.localeCompare(sb) : sb.localeCompare(sa);
  });
}

function updateSortIndicators(table, sort) {
  table.querySelectorAll('.sortable').forEach(th => {
    th.classList.remove('asc', 'desc');
    if (th.dataset.key === sort.key) {
      th.classList.add(sort.asc ? 'asc' : 'desc');
    }
  });
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
    loadContributors(path);
    loadTrends(path);
    loadBranches(path);
    loadFileTypes(path);
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
  // Apply search filter
  let filtered = commits;
  if (commitFilter) {
    const q = commitFilter.toLowerCase();
    filtered = commits.filter(c =>
      (c.message && c.message.toLowerCase().includes(q)) ||
      (c.author_name && c.author_name.toLowerCase().includes(q)) ||
      (c.hash && c.hash.toLowerCase().includes(q))
    );
    if (filtered.length === 0) {
      commitsBody.innerHTML = '<tr><td colspan="4" style="text-align:center;color:#999;">没有匹配的提交</td></tr>';
      return;
    }
  }
  const sorted = sortData(filtered, commitSort);
  updateSortIndicators(document.querySelector('#panel-commits thead'), commitSort);
  commitsBody.innerHTML = sorted.map(c => `
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
  const sorted = sortData(changes, changeSort);
  updateSortIndicators(document.querySelector('#panel-changes thead'), changeSort);
  body.innerHTML = sorted.map(c => `
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

// ── Branch display ─────────────────────────────────────────
async function loadBranches(path) {
  const list = $('branches-list');
  try {
    const data = await cmd('get_branches', { path });
    renderBranches(list, data);
  } catch (e) {
    list.innerHTML = '<div class="branch-empty">加载分支失败：' + escapeHtml(String(e)) + '</div>';
  }
}

function renderBranches(container, branches) {
  if (!branches || branches.length === 0) {
    container.innerHTML = '<div class="branch-empty">暂无分支信息</div>';
    return;
  }
  const locals = branches.filter(b => !b.is_remote);
  const remotes = branches.filter(b => b.is_remote);

  let html = '';
  if (locals.length) {
    html += '<div class="branch-section-title">本地分支</div>';
    html += locals.map(b => `
      <div class="branch-row ${b.is_head ? 'branch-current' : ''}">
        <span class="branch-name">${escapeHtml(b.name)}</span>
        ${b.is_head ? '<span class="branch-tag">当前</span>' : ''}
      </div>
    `).join('');
  }
  if (remotes.length) {
    html += '<div class="branch-section-title">远程分支</div>';
    html += remotes.map(b => `
      <div class="branch-row">
        <span class="branch-name">${escapeHtml(b.name)}</span>
      </div>
    `).join('');
  }
  container.innerHTML = html;
}

// ── File type distribution (from snapshot data) ────────────
async function loadFileTypes(path) {
  const container = $('languages-chart');
  try {
    const entries = await cmd('get_file_types', { path });
    renderFileTypes(container, entries);
  } catch (e) {
    container.innerHTML = '<div class="trend-empty">加载失败：' + escapeHtml(String(e)) + '</div>';
  }
}

function renderFileTypes(container, entries) {
  if (!entries || entries.length === 0) {
    container.innerHTML = '<div class="trend-empty">暂无文件类型数据</div>';
    return;
  }

  if (langChartMode === 'pie') {
    renderFileTypesPie(container, entries);
  } else {
    renderFileTypesBar(container, entries);
  }
}

function renderFileTypesBar(container, entries) {
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

function renderFileTypesPie(container, entries) {
  const PIE_COLORS = ['#4361ee','#43a047','#f9a825','#e53935','#7b2ff7',
                      '#ff6b6b','#20c997','#fd7e14','#6f42c3','#e83e8c',
                      '#17a2b8','#6610f2','#e0a800','#28a745','#dc3545'];
  const total = entries.reduce((s, [,c]) => s + c, 0);
  const size = 220, cx = size / 2, cy = size / 2, r = size / 2 - 10;

  // Build slices
  let startDeg = 0;
  const slices = [];
  entries.slice(0, 12).forEach(([ext, count], i) => {
    const sliceDeg = (count / total) * 360;
    const sr = ((startDeg - 90) * Math.PI) / 180;
    const er = ((startDeg + sliceDeg - 90) * Math.PI) / 180;
    const x1 = cx + r * Math.cos(sr);
    const y1 = cy + r * Math.sin(sr);
    const x2 = cx + r * Math.cos(er);
    const y2 = cy + r * Math.sin(er);
    const large = sliceDeg > 180 ? 1 : 0;
    const d = `M${cx},${cy} L${x1},${y1} A${r},${r} 0 ${large},1 ${x2},${y2} Z`;
    const pct = ((count / total) * 100).toFixed(1);
    slices.push({ ext, count, pct, d, color: PIE_COLORS[i % PIE_COLORS.length] });
    startDeg += sliceDeg;
  });

  const svg = `<svg viewBox="0 0 ${size} ${size}" xmlns="http://www.w3.org/2000/svg">
    ${slices.map(s => `<path d="${s.d}" fill="${s.color}" stroke="#fff" stroke-width="2">
      <title>${escapeHtml(s.ext)}: ${s.count} (${s.pct}%)</title>
    </path>`).join('')}
  </svg>`;

  const legend = slices.map(s => `
    <div class="pie-legend-item">
      <span class="pie-legend-dot" style="background:${s.color}"></span>
      <span>${escapeHtml(s.ext)}</span>
      <span class="pie-legend-value">${s.count}</span>
    </div>
  `).join('');

  container.innerHTML = `<div class="pie-chart">${svg}<div class="pie-legend">${legend}</div></div>`;
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

  if (trendChartMode === 'bar') {
    renderTrendBars(chart, data, isChurn);
  } else if (trendChartMode === 'line') {
    renderTrendLine(chart, data, isChurn);
  } else {
    renderTrendCurve(chart, data, isChurn);
  }
}

function renderTrendBars(chart, data, isChurn) {
  if (isChurn) {
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

function renderTrendLine(chart, data, isChurn) {
  if (!data || data.length < 2) {
    chart.innerHTML = '<div class="trend-empty">数据点不足，无法显示折线图</div>';
    return;
  }
  const W = Math.max(data.length * 30, 300);
  const H = 180, pad = { top: 10, bottom: 28, left: 50, right: 16 };
  const plotW = W - pad.left - pad.right;
  const plotH = H - pad.top - pad.bottom;

  let maxVal, points;
  if (isChurn) {
    maxVal = Math.max(...data.flatMap(([, ins, del]) => [ins, del])) || 1;
    points = data.map(([date, ins, del], i) => ({
      x: pad.left + (i / (data.length - 1)) * plotW,
      y1: pad.top + plotH - (ins / maxVal) * plotH,
      y2: pad.top + plotH - (del / maxVal) * plotH, date, ins, del
    }));
  } else {
    maxVal = Math.max(...data.map(([, c]) => c), 1);
    points = data.map(([date, count], i) => ({
      x: pad.left + (i / (data.length - 1)) * plotW,
      y: pad.top + plotH - (count / maxVal) * plotH, date, count
    }));
  }

  const yTicks = 4;
  const gridLines = Array.from({ length: yTicks + 1 }, (_, i) => {
    const y = pad.top + (plotH / yTicks) * i;
    const v = Math.round(maxVal - (maxVal / yTicks) * i);
    return `<line x1="${pad.left}" y1="${y}" x2="${W - pad.right}" y2="${y}" stroke="#eee" stroke-width="1"/>
      <text x="${pad.left - 6}" y="${y + 4}" text-anchor="end" font-size="10" fill="#999">${v}</text>`;
  }).join('');

  if (isChurn) {
    const insPoints = points.map(p => `${p.x},${p.y1}`).join(' ');
    const delPoints = points.map(p => `${p.x},${p.y2}`).join(' ');
    chart.innerHTML = `<div class="trend-svg-wrap"><svg class="trend-svg" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
      ${gridLines}
      <text x="${pad.left + 6}" y="${pad.top - 2}" font-size="11" fill="#43a047" font-weight="bold">新增</text>
      <text x="${pad.left + 50}" y="${pad.top - 2}" font-size="11" fill="#e53935" font-weight="bold">删除</text>
      <polyline points="${insPoints}" fill="none" stroke="#43a047" stroke-width="2"/>
      <polyline points="${delPoints}" fill="none" stroke="#e53935" stroke-width="2"/>
      ${points.map(p => `<circle cx="${p.x}" cy="${p.y1}" r="3" fill="#43a047"><title>${escapeHtml(p.date)}: +${p.ins}</title></circle>
        <circle cx="${p.x}" cy="${p.y2}" r="3" fill="#e53935"><title>${escapeHtml(p.date)}: -${p.del}</title></circle>`).join('')}
      ${points.map((p, i) => i % Math.ceil(data.length / 8) === 0 || i === data.length - 1
        ? `<text x="${p.x}" y="${H - 6}" text-anchor="end" transform="rotate(-30,${p.x},${H - 6})" font-size="9" fill="#888">${escapeHtml(p.date.slice(5))}</text>`
        : '').join('')}
    </svg></div>`;
  } else {
    const pts = points.map(p => `${p.x},${p.y}`).join(' ');
    chart.innerHTML = `<div class="trend-svg-wrap"><svg class="trend-svg" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
      ${gridLines}
      <polyline points="${pts}" fill="none" stroke="#4361ee" stroke-width="2"/>
      ${points.map(p => `<circle cx="${p.x}" cy="${p.y}" r="3" fill="#4361ee"><title>${escapeHtml(p.date)}: ${p.count} 次提交</title></circle>`).join('')}
      ${points.map((p, i) => i % Math.ceil(data.length / 8) === 0 || i === data.length - 1
        ? `<text x="${p.x}" y="${H - 6}" text-anchor="end" transform="rotate(-30,${p.x},${H - 6})" font-size="9" fill="#888">${escapeHtml(p.date.slice(5))}</text>`
        : '').join('')}
    </svg></div>`;
  }
}

function renderTrendCurve(chart, data, isChurn) {
  if (!data || data.length < 2) {
    chart.innerHTML = '<div class="trend-empty">数据点不足，无法显示曲线图</div>';
    return;
  }
  const W = Math.max(data.length * 30, 300);
  const H = 180, pad = { top: 10, bottom: 28, left: 50, right: 16 };
  const plotW = W - pad.left - pad.right;
  const plotH = H - pad.top - pad.bottom;

  function smoothPath(points) {
    if (points.length < 2) return '';
    let d = `M${points[0].x},${points[0].y}`;
    for (let i = 0; i < points.length - 1; i++) {
      const mx = (points[i].x + points[i + 1].x) / 2;
      const my = (points[i].y + points[i + 1].y) / 2;
      d += ` Q${points[i].x},${points[i].y} ${mx},${my}`;
    }
    d += ` L${points[points.length - 1].x},${points[points.length - 1].y}`;
    return d;
  }

  let maxVal, series1, series2;
  if (isChurn) {
    maxVal = Math.max(...data.flatMap(([, ins, del]) => [ins, del])) || 1;
    series1 = data.map(([date, ins], i) => ({ x: pad.left + (i / (data.length - 1)) * plotW, y: pad.top + plotH - (ins / maxVal) * plotH, date, val: ins }));
    series2 = data.map(([date, , del], i) => ({ x: pad.left + (i / (data.length - 1)) * plotW, y: pad.top + plotH - (del / maxVal) * plotH, date, val: del }));
  } else {
    maxVal = Math.max(...data.map(([, c]) => c), 1);
    series1 = data.map(([date, count], i) => ({ x: pad.left + (i / (data.length - 1)) * plotW, y: pad.top + plotH - (count / maxVal) * plotH, date, val: count }));
    series2 = null;
  }

  const yTicks = 4;
  const gridLines = Array.from({ length: yTicks + 1 }, (_, i) => {
    const y = pad.top + (plotH / yTicks) * i;
    const v = Math.round(maxVal - (maxVal / yTicks) * i);
    return `<line x1="${pad.left}" y1="${y}" x2="${W - pad.right}" y2="${y}" stroke="#eee" stroke-width="1"/>
      <text x="${pad.left - 6}" y="${y + 4}" text-anchor="end" font-size="10" fill="#999">${v}</text>`;
  }).join('');

  const path1 = smoothPath(series1);
  const path2 = series2 ? smoothPath(series2) : '';

  if (isChurn) {
    chart.innerHTML = `<div class="trend-svg-wrap"><svg class="trend-svg" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
      ${gridLines}
      <text x="${pad.left + 6}" y="${pad.top - 2}" font-size="11" fill="#43a047" font-weight="bold">新增</text>
      <text x="${pad.left + 50}" y="${pad.top - 2}" font-size="11" fill="#e53935" font-weight="bold">删除</text>
      <path d="${path1}" fill="none" stroke="#43a047" stroke-width="2"/>
      <path d="${path2}" fill="none" stroke="#e53935" stroke-width="2"/>
      ${series1.map(p => `<circle cx="${p.x}" cy="${p.y}" r="3" fill="#43a047"><title>${escapeHtml(p.date)}: +${p.val}</title></circle>`).join('')}
      ${series2.map(p => `<circle cx="${p.x}" cy="${p.y}" r="3" fill="#e53935"><title>${escapeHtml(p.date)}: -${p.val}</title></circle>`).join('')}
      ${series1.map((p, i) => i % Math.ceil(data.length / 8) === 0 || i === data.length - 1
        ? `<text x="${p.x}" y="${H - 6}" text-anchor="end" transform="rotate(-30,${p.x},${H - 6})" font-size="9" fill="#888">${escapeHtml(p.date.slice(5))}</text>`
        : '').join('')}
    </svg></div>`;
  } else {
    chart.innerHTML = `<div class="trend-svg-wrap"><svg class="trend-svg" viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg">
      ${gridLines}
      <path d="${path1}" fill="none" stroke="#4361ee" stroke-width="2"/>
      ${series1.map(p => `<circle cx="${p.x}" cy="${p.y}" r="3" fill="#4361ee"><title>${escapeHtml(p.date)}: ${p.val} 次提交</title></circle>`).join('')}
      ${series1.map((p, i) => i % Math.ceil(data.length / 8) === 0 || i === data.length - 1
        ? `<text x="${p.x}" y="${H - 6}" text-anchor="end" transform="rotate(-30,${p.x},${H - 6})" font-size="9" fill="#888">${escapeHtml(p.date.slice(5))}</text>`
        : '').join('')}
    </svg></div>`;
  }
}

// ── Export ──────────────────────────────────────────────────
async function doExport(fmt) {
  if (!selectedPath) { setStatus('请先选择一个仓库', true); return; }
  try {
    const msg = await cmd('export_' + fmt, { path: selectedPath });
    setStatus(msg);
  } catch (e) {
    setStatus('导出失败：' + e, true);
  }
}

// ── Settings ───────────────────────────────────────────────
async function saveSettings() {
  settings.max_recent = parseInt($('settings-max-recent').value) || 10;
  settings.max_commits = parseInt($('settings-max-commits').value) || 5000;
  try {
    await cmd('update_settings', { newSettings: settings });
    $('settings-overlay').style.display = 'none';
    setStatus('设置已保存');
  } catch (e) {
    setStatus('保存设置失败：' + e, true);
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
