"use strict";

function profileFromRows(rows) {
  const result = {};
  for (const [rawSource, rawTarget] of rows) {
    const source = rawSource.trim();
    const target = rawTarget.trim();
    if (!source && !target) continue;
    if (!source || !target) throw new Error('Complete both fields in each binding row.');
    if (Object.hasOwn(result, source)) throw new Error(`Duplicate source: ${source}`);
    result[source] = target;
  }
  return result;
}

function canApply(status, preview, savedRevision, currentRevision, busy) {
  return Boolean(status?.live_activation_enabled && !status.focus?.mutation_recovery_pending &&
    !busy && preview?.instances?.length &&
    savedRevision === currentRevision &&
    preview.instances.every(item => item.status === 'ready' || item.status === 'unchanged'));
}

if (typeof module !== 'undefined') module.exports = { canApply, profileFromRows };

if (typeof document !== 'undefined') {
  const $ = id => document.getElementById(id);
  const state = {
    token: '', status: null, preview: null, savedProfile: null,
    revision: 0, savedRevision: -1, busy: false
  };
  const fields = {
    controls: ['Action ID', 'Physical key'],
    crafting: ['Physical key', 'Emitted key'],
    ninjabrain: ['Physical key', 'Ninjabrain shortcut']
  };

  function element(tag, className, content) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (content !== undefined) node.textContent = String(content);
    return node;
  }

  function message(target, value, kind = '') {
    const box = $(target);
    box.replaceChildren();
    if (value) box.append(element('p', `message ${kind}`, value));
  }

  function changed() {
    state.revision++;
    state.preview = null;
    $('preview-results').replaceChildren();
    updateApply();
  }

  function updateApply() {
    $('apply-profile').disabled = !canApply(
      state.status, state.preview, state.savedRevision, state.revision, state.busy
    );
  }

  function rowInput(label, value = '') {
    const wrapper = element('label');
    wrapper.append(element('span', 'sr-only', label));
    const input = element('input');
    input.type = 'text';
    input.value = value;
    input.placeholder = label;
    input.setAttribute('aria-label', label);
    wrapper.append(input);
    return wrapper;
  }

  function addMappingRow(section, source = '', target = '', silent = false) {
    const row = element('div', 'mapping-row');
    row.append(rowInput(fields[section][0], source), rowInput(fields[section][1], target));
    const remove = element('button', 'remove', 'Remove');
    remove.type = 'button';
    remove.setAttribute('aria-label', `Remove ${section} row`);
    remove.addEventListener('click', () => { row.remove(); changed(); });
    row.append(remove);
    $(`${section}-rows`).append(row);
    if (!silent) changed();
  }

  function addPresetRow(preset = {}, silent = false) {
    const row = element('div', 'preset-row');
    for (const [key, label, type] of [
      ['id', 'Preset name', 'text'], ['width', 'Width', 'number'],
      ['height', 'Height', 'number'], ['shortcut', 'Shortcut', 'text']
    ]) {
      const wrapper = rowInput(label, preset[key] ?? '');
      const input = wrapper.querySelector('input');
      input.dataset.field = key;
      input.type = type;
      input.required = true;
      if (type === 'number') { input.min = '100'; input.max = '16384'; }
      row.append(wrapper);
    }
    const eye = element('label', 'eye-label');
    const check = element('input');
    check.type = 'checkbox';
    check.dataset.field = 'eye_zoom';
    check.checked = Boolean(preset.eye_zoom);
    eye.append(check, document.createTextNode(' Eye zoom'));
    row.append(eye);
    const remove = element('button', 'remove', 'Remove');
    remove.type = 'button';
    remove.setAttribute('aria-label', 'Remove preset');
    remove.addEventListener('click', () => { row.remove(); changed(); });
    row.append(remove);
    $('presets-rows').append(row);
    if (!silent) changed();
  }

  function readRows(section) {
    return [...$(`${section}-rows`).children].map(row =>
      [...row.querySelectorAll('input')].map(input => input.value));
  }

  function readProfile() {
    const chat = {};
    for (const name of ['open_chat', 'open_command', 'send', 'cancel']) {
      chat[name] = document.querySelector(`[name="${name}"]`).value.trim();
      if (!chat[name]) throw new Error('Complete all chat keys.');
    }
    const presets = [...$('presets-rows').children].map(row => {
      const get = key => row.querySelector(`[data-field="${key}"]`);
      const width = Number(get('width').value);
      const height = Number(get('height').value);
      if (!get('id').value.trim() || !get('shortcut').value.trim() ||
          !Number.isInteger(width) || !Number.isInteger(height) ||
          width < 100 || height < 100 || width > 16384 || height > 16384) {
        throw new Error('Each preset needs a name, shortcut, and dimensions from 100 to 16384.');
      }
      return {
        id: get('id').value.trim(), width, height,
        shortcut: get('shortcut').value.trim(), eye_zoom: get('eye_zoom').checked
      };
    });
    if (presets.length > 8) throw new Error('Use at most eight presets.');
    return {
      controls: profileFromRows(readRows('controls')),
      crafting: profileFromRows(readRows('crafting')),
      ninjabrain: profileFromRows(readRows('ninjabrain')),
      snippets: $('snippets').value.split(/\r?\n/).map(s => s.trim()).filter(Boolean),
      chat, presets,
      launcher_root: $('launcher-root').value.trim() || null
    };
  }

  function fillProfile(profile) {
    state.savedProfile = profile;
    for (const section of Object.keys(fields)) {
      $(`${section}-rows`).replaceChildren();
      for (const [source, target] of Object.entries(profile[section] || {})) {
        addMappingRow(section, source, target, true);
      }
    }
    $('snippets').value = (profile.snippets || []).join('\n');
    $('launcher-root').value = profile.launcher_root || '';
    for (const name of ['open_chat', 'open_command', 'send', 'cancel']) {
      document.querySelector(`[name="${name}"]`).value = profile.chat?.[name] || '';
    }
    $('presets-rows').replaceChildren();
    for (const preset of profile.presets || []) addPresetRow(preset, true);
    state.revision = 0;
    state.savedRevision = 0;
    state.preview = null;
    updateApply();
  }

  async function api(path, method = 'GET', data) {
    const headers = {};
    if (method !== 'GET') {
      headers.Origin = window.location.origin;
      headers['X-MCSR-Session'] = state.token;
      if (data !== undefined) headers['Content-Type'] = 'application/json';
    }
    const response = await fetch(path, {
      method, headers, credentials: 'same-origin',
      body: data === undefined ? undefined : JSON.stringify(data)
    });
    const body = await response.json();
    if (!response.ok) {
      const detail = body.detail;
      let explanation;
      if (Array.isArray(detail)) explanation = detail.map(item => item.msg).join('; ');
      else explanation = detail?.message || detail?.code || String(detail || response.statusText);
      throw new Error(explanation);
    }
    return body;
  }

  function showStatus(status) {
    state.status = status;
    const grid = $('status');
    grid.replaceChildren();
    const cards = [
      ['Activation', status.live_activation_enabled ? 'Available' : 'Observe only'],
      ['Launcher root', status.launcher_root || 'Not detected'],
      ['Instances', String(status.instances?.length ?? 0)],
      ['Focus', status.focus?.mode || 'Unknown']
    ];
    for (const [label, value] of cards) {
      const card = element('div', 'status-card');
      card.append(element('span', 'status-label', label), element('strong', '', value));
      grid.append(card);
    }
    if (status.instances?.length) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Discovered instances'));
      renderInstances(card, status.instances.map(item => ({
        instance_id: item.id,
        status: item.problem ? 'failed' : 'detected',
        detail: item.problem || ''
      })));
      grid.append(card);
    }
    const focus = status.focus || {};
    const focusDetails = [
      ['Minecraft focus', focus.minecraft_focused],
      ['GNOME extension', focus.extension_connected],
      ['Activation suspended', focus.activation_suspended]
    ].filter(([, value]) => typeof value === 'boolean');
    if (focusDetails.length) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Focus service'));
      for (const [label, value] of focusDetails) {
        card.append(element('p', 'help', `${label}: ${value ? 'Yes' : 'No'}`));
      }
      grid.append(card);
    }
    if (focus.cleanup_error) grid.append(element('p', 'message error status-detail', focus.cleanup_error));
    message('cleanup-warning', status.focus?.unsafe_stop_failure ||
      status.focus?.recovery_clear_pending || status.focus?.mutation_recovery_pending
      ? 'Keyd cleanup needs recovery. Check the service status before activating mappings.'
      : '', 'error');
    if (status.focus?.reconciliation) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Instance reconciliation'));
      renderInstances(card, status.focus.reconciliation.instances || []);
      grid.append(card);
    }
    updateApply();
  }

  function renderInstances(container, instances) {
    const list = element('ul', 'instance-list');
    for (const item of instances) {
      const entry = element('li');
      entry.append(element('strong', '', item.instance_id || 'Unknown instance'));
      entry.append(element('span', `badge ${item.status}`, item.status || 'unknown'));
      if (item.detail) entry.append(element('span', 'detail', item.detail));
      list.append(entry);
    }
    if (!instances.length) container.append(element('p', 'help', 'No instances reported.'));
    else container.append(list);
  }

  function renderPreview(preview) {
    const box = $('preview-results');
    box.replaceChildren(element('h3', '', 'Per-instance preview'));
    renderInstances(box, preview.instances || []);
    if (preview.mappings?.length) {
      const table = element('table');
      const caption = element('caption', '', 'Control mappings by instance');
      const head = element('thead');
      const headRow = element('tr');
      for (const label of ['Instance', 'Physical', 'Emitted', 'Minecraft action']) {
        headRow.append(element('th', '', label));
      }
      head.append(headRow);
      const body = element('tbody');
      for (const mapping of preview.mappings) {
        const row = element('tr');
        for (const value of [mapping.instance_id, mapping.physical, mapping.emitted, mapping.action]) {
          row.append(element('td', '', value ?? ''));
        }
        body.append(row);
      }
      table.append(caption, head, body);
      box.append(table);
    }
    if (preview.instances?.some(item => !['ready', 'unchanged'].includes(item.status))) {
      box.append(element('p', 'message error', 'Apply is blocked until every instance is ready or unchanged.'));
    }
  }

  function renderJob(target, result) {
    const box = $(target);
    box.replaceChildren();
    if (result.error) box.append(element('p', 'message error', result.error));
    if (result.instances) renderInstances(box, result.instances);
    if (result.keyd_error) box.append(element('p', 'message error', `keyd: ${result.keyd_error}`));
    if (result.backup_id) {
      box.append(element('p', 'message', `Backup ID: ${result.backup_id}`));
      if (target === 'apply-results') $('backup-id').value = result.backup_id;
    }
    if (result.version || result.path) {
      box.append(element('p', 'message', `Version: ${result.version || 'unknown'} · Saved to: ${result.path || 'unknown'}`));
    }
  }

  async function runJob(path, target, data) {
    const accepted = await api(path, 'POST', data);
    message(target, `Job ${accepted.id} is running…`);
    for (;;) {
      await new Promise(resolve => setTimeout(resolve, 700));
      const job = await api(`/api/jobs/${encodeURIComponent(accepted.id)}`);
      if (job.status === 'running') continue;
      renderJob(target, job.status === 'failed' ? { error: job.error || 'Job failed.' } : job.result || {});
      await refreshStatus();
      return job;
    }
  }

  async function refreshStatus() { showStatus(await api('/api/status')); }

  async function action(work, errorTarget = 'notice') {
    if (state.busy) return;
    state.busy = true;
    updateApply();
    try { await work(); }
    catch (error) { message(errorTarget, error.message, 'error'); }
    finally { state.busy = false; updateApply(); }
  }

  async function init() {
    try {
      state.token = (await api('/api/session')).token;
      const [profile, status] = await Promise.all([api('/api/profile'), api('/api/status')]);
      fillProfile(profile);
      showStatus(status);
    } catch (error) { message('notice', `Could not load toolkit: ${error.message}`, 'error'); }
  }

  $('profile-form').addEventListener('submit', event => event.preventDefault());
  $('profile-form').addEventListener('input', changed);
  $('profile-form').addEventListener('change', event => {
    if (event.target.type === 'checkbox') changed();
  });
  $('launcher-root').addEventListener('input', changed);
  for (const button of document.querySelectorAll('[data-add]')) {
    button.addEventListener('click', () => {
      if (button.dataset.add === 'presets') addPresetRow();
      else addMappingRow(button.dataset.add);
    });
  }
  $('refresh-status').addEventListener('click', () => action(refreshStatus));
  $('save-profile').addEventListener('click', () => action(async () => {
    const revision = state.revision;
    const saved = await api('/api/profile', 'PUT', readProfile());
    state.savedProfile = saved;
    state.savedRevision = revision;
    state.preview = null;
    $('preview-results').replaceChildren();
    message('notice', 'Profile saved. Preview changes before applying.');
    await refreshStatus();
  }));
  $('preview-profile').addEventListener('click', () => action(async () => {
    const revision = state.revision;
    state.preview = null;
    updateApply();
    const preview = await api('/api/preview', 'POST', readProfile());
    state.preview = revision === state.revision ? preview : null;
    if (state.preview) renderPreview(preview);
    if (state.savedRevision !== state.revision) message('notice', 'Preview is for an unsaved draft. Save this profile before Apply.');
  }, 'preview-results'));
  $('apply-profile').addEventListener('click', () => action(async () => {
    if (!canApply(state.status, state.preview, state.savedRevision, state.revision, false)) return;
    let job;
    try { job = await runJob('/api/apply', 'apply-results', state.savedProfile); }
    catch (error) {
      state.preview = null;
      $('preview-results').replaceChildren();
      throw error;
    }
    state.preview = null;
    updateApply();
    if (job.status === 'succeeded') message('notice', 'Apply job finished. Review each instance result.');
  }, 'apply-results'));
  $('restore-backup').addEventListener('click', () => action(async () => {
    const backupId = $('backup-id').value.trim();
    if (!backupId) throw new Error('Enter a backup ID.');
    const revision = state.revision;
    const wasSaved = state.savedRevision === revision;
    const job = await runJob('/api/restore', 'restore-results', { backup_id: backupId });
    state.preview = null;
    $('preview-results').replaceChildren();
    if (job.status === 'succeeded') {
      const restored = await api('/api/profile');
      if (wasSaved && state.revision === revision) {
        fillProfile(restored);
        message('notice', 'Restored the saved profile. Preview changes before applying.');
      } else {
        state.savedProfile = restored;
        state.savedRevision = -1;
        message('notice', 'Restore finished. Your unsaved draft was preserved; save and preview before Apply.');
      }
    }
  }, 'restore-results'));
  for (const button of document.querySelectorAll('[data-download]')) {
    button.addEventListener('click', () => action(async () => {
      await runJob(`/api/download/${button.dataset.download}`, `download-${button.dataset.download}-results`);
    }, `download-${button.dataset.download}-results`));
  }
  init();
}
