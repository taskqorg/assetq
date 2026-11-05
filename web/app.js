'use strict';
// Browser state is a projection of the scoped API; no durable asset/job store.
const main = document.querySelector('#main');
let session, project, epoch = 0, pollTimer, lastView;
const drafts = new Map(), views = new Map();
const terminal = p => ['succeeded', 'failed', 'cancelled'].includes(p);
const value = v => v === null || v === undefined ? 'Unknown' : typeof v === 'object' ? JSON.stringify(v, null, 2) : String(v);
const short = id => id ? `${id.slice(0, 8)}…` : 'Unknown';
function el(tag, attrs = {}, ...children) {
  const n = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k.startsWith('on')) n.addEventListener(k.slice(2), v);
    else if (k === 'class') n.className = v;
    else if (k === 'checked' || k === 'disabled' || k === 'selected') n[k] = v;
    else if (v !== undefined && v !== null) n.setAttribute(k, v);
  }
  n.append(...children.flat().filter(c => c !== null && c !== undefined).map(c => c instanceof Node ? c : document.createTextNode(String(c))));
  return n;
}
const note = message => el('div', {class: 'notice', role: 'status'}, message);
const panel = (...children) => el('section', {class: 'panel'}, ...children);
const heading = (title, ...actions) => el('div', {class: 'heading'}, el('h1', {}, title), el('div', {class: 'actions'}, ...actions));
const button = (label, action, primary = false) => el('button', {type: 'button', class: primary ? 'primary' : '', onclick: action}, label);
function url(path, params = {}) {
  const u = new URL(path, location.origin);
  u.searchParams.set('project', project);
  for (const [k, v] of Object.entries(params)) if (v !== null && v !== undefined && v !== '') u.searchParams.set(k, v);
  return u.pathname + u.search;
}
function link(label, path, params) { return el('a', {href: url(path, params)}, label); }
function jsonDetails(title, data, open = false) { return el('details', open ? {open: ''} : {}, el('summary', {}, title), el('pre', {}, value(data))); }
function properties(values) { return el('dl', {class: 'properties'}, Object.entries(values).flatMap(([k, v]) => [el('dt', {}, k), el('dd', {}, v instanceof Node ? v : value(v))])); }
function role() { return session?.projects.find(p => p.id === project)?.role; }
function requireOwner(control) { return role() === 'owner' ? control : el('p', {class: 'muted'}, 'An owner grant is required for this action.'); }
function loseSession(message) {
  ++epoch; clearTimeout(pollTimer); session = null; lastView = null; drafts.clear(); views.clear();
  document.querySelector('#project').replaceChildren(); document.querySelector('#identity').textContent = '';
  main.replaceChildren(heading('Access unavailable'), note(message), button('Reconnect through host sign-in', () => bootstrap()));
}
async function api(path, {method = 'GET', body, binary = false} = {}) {
  if (!session) throw new Error('No authorized browser session.');
  const scope = project, identity = session;
  const u = AssetQRequest.apiUrl(path, project, location.origin);
  const headers = {};
  if (method !== 'GET') headers['X-CSRF-Token'] = session.csrf;
  if (body !== undefined && !binary) headers['Content-Type'] = 'application/json';
  if (binary) { headers['Content-Type'] = body.type || 'application/octet-stream'; headers['X-Asset-Name-UTF8'] = encodeURIComponent(body.name); }
  const response = await fetch(u, {method, headers, credentials: 'same-origin', cache: 'no-store', body: body === undefined ? undefined : binary ? body : JSON.stringify(body)});
  const data = await response.json();
  if (scope !== project || identity !== session) throw new Error('The project or session changed; the old response was discarded.');
  if (!response.ok) {
    if (data.error === 'browser_denied') loseSession('The browser session or project authority is unavailable. Private data was cleared.');
    const error = new Error(`${data.error}: ${data.reason}`); error.status = response.status; throw error;
  }
  return data;
}
const items = data => Array.isArray(data) ? data : data.items || [];
function pageLink(path, data, params = {}) { return data.next ? el('p', {class: 'paging'}, link('Next page →', path, {...params, after: data.next})) : null; }
function preview(object, compact = false) {
  const box = el('div', {class: compact ? 'thumb' : 'preview'});
  const caption = !compact && object.content_type === 'model/gltf-binary' ? el('small', {}, 'Retained render of this job; not a live 3D view.') : null;
  const fallback = () => { caption?.remove(); box.replaceChildren(el('div', {class: 'file-fallback', title: object.content_type}, compact ? 'File' : object.content_type, compact ? null : el('small', {}, 'Preview unavailable; download the original.'))); };
  if (object.content_type !== 'image/png' && object.content_type !== 'model/gltf-binary') { fallback(); return box; }
  const img = el('img', {src: url(`/browser/v1/objects/${encodeURIComponent(object.id)}/preview`), alt: object.provenance?.name || `Retained PNG ${object.id}`, loading: 'lazy', onload: () => { if (caption) box.after(caption); }, onerror: fallback});
  box.append(img); return box;
}
function download(object) { return link('Download original', `/browser/v1/objects/${encodeURIComponent(object.id)}/content`); }
function objectName(o) { return o.provenance?.name || o.provenance?.import_key || `${o.kind} ${short(o.id)}`; }
function rememberDraft() {
  for (const form of main.querySelectorAll('form.editor')) {
    const data = new FormData(form), saved = {};
    for (const key of new Set(data.keys())) saved[key] = data.getAll(key).filter(v => typeof v === 'string');
    drafts.set(form.dataset.draftKey, saved);
  }
}
function navigate(path, replace = false) {
  rememberDraft(); history.replaceState({...history.state, scroll: scrollY, focus:document.activeElement?.getAttribute('href')}, '', location.href);
  const next = new URL(path,location.origin), current = new URL(location.href);
  const preserve = next.pathname === '/assets' && current.pathname === next.pathname && ['project','q','type','after'].every(k=>next.searchParams.get(k)===current.searchParams.get(k));
  history[replace ? 'replaceState' : 'pushState']({scroll: preserve ? scrollY : 0}, '', path); render(preserve);
}
document.addEventListener('click', e => {
  const a = e.target.closest('a');
  if (!a || e.defaultPrevented || e.button || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
  const u = new URL(a.href); if (u.origin !== location.origin || u.pathname.startsWith('/browser/') || u.hash) return;
  e.preventDefault(); navigate(u.pathname + u.search);
});
addEventListener('popstate', () => render(true));

async function assetDetail(id, token) {
  const [o, related] = await Promise.all([api(`objects/${encodeURIComponent(id)}`), api(`objects/${encodeURIComponent(id)}/related?` + new URLSearchParams({after:new URLSearchParams(location.search).get('after') || ''}))]);
  if (token !== epoch) return null;
  const box = panel(el('h2', {}, objectName(o)), el('p', {class: 'muted'}, `${o.content_type} / ${o.size.toLocaleString()} bytes`), preview(o),
    el('div', {class: 'actions'}, download(o), o.job_id ? link('Open job', `/jobs/${o.job_id}`) : null),
    properties({'Validation': o.provenance?.provider_evidence?.validation?.valid === true ? 'Passed' : o.provenance?.provider_evidence?.validation?.valid === false ? 'Failed' : 'Unknown: no validation verdict recorded in provenance', 'Immutable object': o.id, 'SHA-256': o.sha256, 'Owner acceptance': o.acceptance, 'Synthetic': String(o.synthetic), 'Created': o.created_at, 'Retention': o.retention_until || 'No automatic deletion'}));
  const objectLinks = (label, list) => el('section', {}, el('h3', {}, label), list.length ? el('ul', {}, list.map(x => el('li', {}, link(objectName(x), `/assets/${x.id}`)))) : el('p', {class: 'muted'}, 'None recorded.'));
  box.append(objectLinks('Source attachments', related.parents), objectLinks('Committed job outputs', related.outputs), objectLinks('Derived objects / versions', related.children));
  if (related.next_children) box.append(link('More derived objects →', `/assets/${id}`, {after: related.next_children}));
  box.append(jsonDetails('Provenance and validation evidence', o.provenance, true));
  if (role() === 'owner') {
    const select = el('select', {'aria-label': 'Owner acceptance'}, ['pending', 'accepted', 'rejected'].map(s => el('option', {value: s, selected: s === o.acceptance}, s)));
    const status = el('p', {role: 'status'});
    box.append(el('div', {class: 'actions'}, select, button('Save acceptance', async () => {
      try { await api(`objects/${id}/acceptance`, {method: 'POST', body: {acceptance: select.value}}); status.textContent = 'Acceptance recorded.'; }
      catch (e) { status.textContent = e.message; }
    })), status);
  }
  return box;
}

async function assets(params, token, id) {
  if (id) return [heading('Asset detail', link('Asset library', views.get(`${project}:assets`) || '/assets')), await assetDetail(id, token)];
  const q = params.get('q') || '', type = params.get('type') || '', after = params.get('after') || '';
  const page = await api('objects?' + new URLSearchParams({q, type, after, limit: 30}));
  const filter = el('form', {class: 'filters', onsubmit: e => {e.preventDefault(); const data = new FormData(filter); navigate(url('/assets', {q: data.get('q'), type: data.get('type')}));}},
    el('label', {}, 'Search', el('input', {name: 'q', value: q, maxlength: 256, placeholder: 'Name, prompt, method or ID'})),
    el('label', {}, 'Type', el('select', {name: 'type'}, [['', 'All'], ['image', 'Image'], ['model', '3D model'], ['text', 'Text'], ['other', 'Other']].map(([v, label]) => el('option', {value: v, selected: v === type}, label)))),
    el('button', {type: 'submit'}, 'Search'), link('Clear filters', '/assets'));
  const selected = params.get('selected');
  const rows = items(page).map(o => el('article', {class: 'asset-row' + (o.id === selected ? ' selection' : '')}, preview(o, true),
    el('div', {}, link(objectName(o), '/assets', {q, type, after, selected: o.id}), el('small', {}, `${o.id} / ${o.size.toLocaleString()} bytes`)),
    el('div', {}, o.content_type, el('small', {}, `${o.kind} / ${o.acceptance}`))));
  const library = el('section', {}, el('div', {class: 'table-list'}, el('div', {class: 'asset-row asset-head'}, el('span', {'aria-hidden': 'true'}), el('span', {}, 'Asset'), el('span', {}, 'Type / acceptance')), rows.length ? rows : note('No matching assets. Clear the filters or submit an authorized request.')), pageLink('/assets', page, {q, type}));
  let detail = selected ? await assetDetail(selected, token).catch(e => panel(note(e.message))) : panel(el('h2', {}, 'Select an asset'), el('p', {class: 'muted'}, 'Preview, provenance, source relationships and download stay beside your search.'));
  return [heading('Assets', link('New generation', '/jobs/new')), el('p', {class: 'muted'}, 'Find retained originals, renders and their source relationships.'), filter, el('div', {class: 'workspace'}, library, el('aside', {class: 'detail', 'aria-label': 'Selected asset'}, detail))];
}

function field(label, name, initial = '', type = 'text', extra = {}) {
  return el('label', {}, label, el(type === 'textarea' ? 'textarea' : 'input', {name, ...(type === 'textarea' ? {} : {type,value:initial}), ...extra}, type === 'textarea' ? initial : null));
}
function editor(title, fields, submit, label = 'Save', draftId = title) {
  const draftKey = `${project}:${location.pathname}:${draftId}:${new URLSearchParams(location.search).get('method') || ''}`;
  const form = el('form', {class: 'editor panel', 'data-draft-key':draftKey}, fields);
  const draft = drafts.get(draftKey);
  for (const control of form.elements) if (control.name) {
    if (draft && Object.hasOwn(draft, control.name)) {
      if (control.multiple) for (const o of control.options) o.selected = draft[control.name].includes(o.value);
      else control.value = draft[control.name][0] || '';
    }
  }
  const status = el('p', {class: 'error', role: 'status'}), send = el('button', {type: 'submit', class: 'primary'}, label);
  form.append(send, status);
  form.addEventListener('submit', async e => {
    e.preventDefault(); send.disabled = true; status.textContent = 'Saving…'; rememberDraft();
    try { await submit(Object.fromEntries(new FormData(form)), status); }
    catch (error) { if (form.isConnected) status.textContent = error.message; }
    finally { send.disabled = false; }
  });
  return [heading(title), form];
}
const input = field;
function jsonField(label, name, initial) { return field(label, name, JSON.stringify(initial, null, 2), 'textarea', {spellcheck: 'false'}); }
function parse(s, fallback = {}) { return s.trim() ? JSON.parse(s) : fallback; }

async function newJob() {
  const methods = items(await api('methods?limit=500'));
  const pinned = new URLSearchParams(location.search).get('method');
  if (pinned && !methods.some(m => `${m.id}@${m.version}` === pinned)) {
    const match = /^(.+)@([1-9][0-9]*)$/.exec(pinned);
    if (!match) throw new Error('The requested method pin is invalid.');
    methods.push(await api(`methods/${encodeURIComponent(match[1])}/versions/${match[2]}`));
  }
  if (pinned && !methods.some(m => `${m.id}@${m.version}` === pinned)) throw new Error('The requested method version is unavailable. No alternative was selected.');
  const select = el('select', {name: 'method', required: ''}, methods.map(m => el('option', {value: `${m.id}@${m.version}`, selected: pinned === `${m.id}@${m.version}`}, `${m.id}@${m.version} / ${m.status}`)));
  const evidence = el('div');
  const showMethod = () => { const m = methods.find(m => `${m.id}@${m.version}` === select.value); evidence.replaceChildren(note(m?.qualification?.reason || 'Registry presence does not grant dispatch. Existing capability and budget checks apply.'), jsonDetails('Method inputs, parameters and limits', m?.spec)); };
  select.addEventListener('change', showMethod); showMethod();
  const attachments = el('textarea', {name: 'attachments', placeholder: 'One immutable object ID per line', rows: 3});
  const uploadStatus = el('p', {role: 'status'});
  const files = el('input', {type: 'file', multiple: '', 'aria-label': 'Upload attachments', onchange: async () => {
    files.disabled = true;
    try {
      if (files.files.length > 16) throw new Error('At most 16 attachments per request.');
      for (const file of files.files) { uploadStatus.textContent = `Uploading ${file.name}…`; const o = await api('uploads', {method: 'POST', body: file, binary: true}); attachments.value += (attachments.value.trim() ? '\n' : '') + o.id; }
      uploadStatus.textContent = 'Uploaded immutable attachment IDs retained below.';
    } catch (e) { uploadStatus.textContent = e.message; } finally { files.disabled = false; }
  }});
  const storageKey = `assetq-pending-key:${project}`;
  const pending = await AssetQRequest.pendingRequest(sessionStorage,storageKey,key=>api(`jobs/by-key/${encodeURIComponent(key)}`),()=>crypto.randomUUID());
  const key = pending.key;
  if (pending.job) return [heading('Request already accepted'),note('The retained key resolved to the original durable job. A new request now requires an explicit action.'),link('Open original job',`/jobs/${pending.job.id}`),button('Start another request',()=>render())];
  return editor('New generation', [el('label', {}, 'Pinned method', select), evidence, field('Brief', 'text', '', 'textarea', {maxlength: 65536}), el('label', {}, 'Attachments', attachments), files, uploadStatus,
    input('Maximum credits for this job', 'max_credits', '0', 'number', {min: 0, step: 1, required: ''}), jsonField('Method constraints', 'constraints', {}),
    note('Submission uses one retained request key. A lost response stays unknown; retry preserves that key and never authorizes a replacement provider operation.')], async (f, status) => {
      const m = methods.find(m => `${m.id}@${m.version}` === f.method);
      const body = AssetQRequest.job(f, m, key);
      status.textContent = 'Submitting with the retained key…';
      try {
        const result = await api('jobs', {method: 'POST', body});
        sessionStorage.removeItem(storageKey); navigate(url(`/jobs/${result.job.id}`)); for (const key of drafts.keys()) if(key.startsWith(`${project}:/jobs/new:`)) drafts.delete(key);
      } catch (e) { throw new Error(`${e.message}\nIf the response was lost, check Jobs before changing the request. The original key is retained.`); }
    }, 'Submit request');
}

async function jobs(params, token, id) {
  if (id === 'new') return newJob();
  if (id) {
    const [job, events, result] = await Promise.all([api(`jobs/${id}`), api(`jobs/${id}/events`), api(`jobs/${id}/result`)]);
    const live = el('div');
    const show = j => live.replaceChildren(properties({'State': j.phase, 'Step': `${j.step} / ${(j.steps || []).length}`, 'Cancellation': j.cancel, 'Reason': j.reason || 'None recorded', 'Credits': j.usage?.credits, 'Usage evidence': j.usage?.unknown_reason || 'Reported', 'Observed': j.updated_at, 'Provider operation': j.provider_operation}));
    show(job);
    const status = el('p', {role: 'status'});
    const cancel = button('Request cancellation', async () => { cancel.disabled = true; try { const j = await api(`jobs/${id}/cancel`, {method: 'POST'}); show(j); status.textContent = 'Cancellation requested; provider and descendant completion require readback.'; } catch(e) {status.textContent=e.message;} finally{cancel.disabled=false;} });
    const refresh = async () => {
      try { const j = await api(`jobs/${id}`); if (token !== epoch) return; show(j); status.textContent = ''; if (!terminal(j.phase)) pollTimer = setTimeout(refresh, 5000); else status.replaceChildren('Terminal state observed. ', button('Read results and history', () => render())); }
      catch(e) { if (token === epoch) status.textContent = `Observation unknown: ${e.message}. Last-known values above are stale.`; }
    };
    if (!terminal(job.phase)) pollTimer = setTimeout(refresh, 5000);
    const outputs = result.manifest?.items || [];
    return [heading(`Job ${short(id)}`, link('Job history', '/jobs'), button('Refresh', () => render())), panel(el('p', {}, id), live, !terminal(job.phase) ? cancel : null, status),
      panel(el('h2', {}, 'Results'), outputs.length ? el('ul', {}, outputs.map(o => el('li', {}, link(o.content_type + ' / ' + short(o.object_id), `/assets/${o.object_id}`)))) : note('No verified success manifest. Retained failed-import objects remain visible in Assets.')),
      panel(jsonDetails('Pinned request and method', {method: job.method, request: job.request}, true), jsonDetails('Durable event history', events), jsonDetails('Result and validation', result))];
  }
  const phase = params.get('phase') || '', after = params.get('after') || '';
  const page = await api('jobs?' + new URLSearchParams({phase, after, limit: 50}));
  const select = el('select', {'aria-label': 'Job phase', onchange: () => navigate(url('/jobs', {phase: select.value}))}, ['', 'queued', 'active', 'waiting', 'succeeded', 'failed', 'cancelled'].map(v => el('option', {value: v, selected: phase === v}, v || 'All phases')));
  return [heading('Jobs', link('New generation', '/jobs/new')), el('div', {class: 'filters'}, select), panel(items(page).length ? items(page).map(j => el('div', {class: 'row'}, link(`${j.phase} / ${j.method}`, `/jobs/${j.id}`), el('small', {}, ` ${j.id}`), el('p', {class: 'muted'}, j.reason || `Credits: ${value(j.usage?.credits)} / ${j.updated_at}`))) : note('No jobs match this view.')), pageLink('/jobs', page, {phase})];
}

async function methods(params, token, id, version) {
  if (id) {
    const method = await api(`methods/${id}/versions/${version}`);
    const edit = editor('Create method version', [note('The server creates a new version; existing jobs and original method revisions stay unchanged.'), input('Lifecycle status','status',method.status), jsonField('Version specification', 'spec', method.spec)], async f => { const m = await api('methods', {method: 'POST', body: AssetQRequest.method(f,method.id)}); navigate(url(`/methods/${encodeURIComponent(m.id)}/versions/${m.version}`)); }, 'Create version')[1];
    return [heading(`${method.id}@${method.version}`, link('Methods', '/methods'), link('New generation', '/jobs/new', {method: `${method.id}@${method.version}`})),
      panel(properties({'Status':method.status,'Operation':method.spec.operation,'Provider':method.spec.provider,'API / model':method.spec.model || method.spec.api || 'See specification'}), jsonDetails('Qualification and limitations', method.qualification, true), jsonDetails('Pipeline, parameters and validation recipe', method.spec, true)), requireOwner(edit)];
  }
  const page = await api('methods?' + new URLSearchParams({operation: params.get('operation') || '', after: params.get('after') || '', limit: 100}));
  const list = items(page);
  const filter = el('form', {class:'filters', onsubmit:e=>{e.preventDefault();navigate(url('/methods',{operation:new FormData(filter).get('operation')}));}}, input('Operation','operation',params.get('operation')||''), el('button',{type:'submit'},'Filter'));
  const add = editor('Add a versioned method', [input('Method identity','id',''), input('Lifecycle status','status','experimental'), jsonField('Method specification (review all copied fields)', 'spec', list[0]?.spec || {})], async f=>{const m=await api('methods',{method:'POST',body:AssetQRequest.method(f)});navigate(url(`/methods/${encodeURIComponent(m.id)}/versions/${m.version}`));},'Create method')[1];
  return [heading('Methods'),filter,panel(list.length ? list.map(m=>el('div',{class:'row'},link(`${m.id}@${m.version}`,`/methods/${encodeURIComponent(m.id)}/versions/${m.version}`),el('p',{},`${m.status} / ${m.spec.operation} / ${m.spec.provider}`),el('small',{},m.qualification?.reason || 'No qualification evidence.'))) : note('No methods match this scope.')),pageLink('/methods',page,{operation:params.get('operation')}),requireOwner(el('details',{},el('summary',{},'Add method'),add))];
}

async function benchmarkSetup() {
  const [datasetPage,evaluatorPage,methodPage]=await Promise.all([api('datasets'),api('evaluators'),api('methods')]);
  const datasets=items(datasetPage),evaluators=items(evaluatorPage),methods=items(methodPage);
  const revisions=(name,list)=>el('select',{name,required:''},list.map(r=>el('option',{value:`${r.id}@${r.version}`},`${r.id}@${r.version}`)));
  const methodSelect=el('select',{name:'methods',multiple:'',required:'',size:Math.min(6,methods.length)},methods.map(m=>el('option',{value:`${m.id}@${m.version}`},`${m.id}@${m.version} / ${m.spec.operation}`)));
  const matrix=el('p',{role:'status'});
  const fields=[input('Use case','use_case',''),el('label',{},'Dataset revision',revisions('dataset',datasets)),el('label',{},'Evaluator revision',revisions('evaluator',evaluators)),el('label',{},'Pinned candidates (select one or more)',methodSelect),input('Repeats per case','repeats','1','number',{min:1,max:8}),input('Maximum jobs','max_jobs','4','number',{min:1,max:32}),input('Maximum credits','max_credits','0','number',{min:0}),jsonField('Exact method constraints','constraints',{}),matrix,note('Running comparisons currently admits only explicitly synthetic methods. A real-provider method cannot be enabled by this form.')];
  const result=editor('New benchmark',fields,async f=>{
    const pins=[...methodSelect.selectedOptions].map(o=>o.value);
    const comparison=await api('comparisons',{method:'POST',body:AssetQRequest.comparison(f,pins)});
    navigate(url(`/benchmarks/${comparison.id}`));
  },'Create frozen comparison');
  const form=result[1];
  const count=()=>{const d=datasets.find(d=>`${d.id}@${d.version}`===form.elements.dataset.value);matrix.textContent=`Planned samples: ${(d?.document?.cases?.length || 0)*methodSelect.selectedOptions.length*Number(form.elements.repeats.value)}. The server enforces operation, revision, job and credit limits before admission.`;};
  form.addEventListener('change',count);count();return result;
}

async function benchmarks(params, token, id) {
  if(id==='new') return benchmarkSetup();
  if(!id){
    const page=await api('comparisons?'+new URLSearchParams({limit:50,after:params.get('after')||''}));
    const revisionEditor=(kind,initial)=>editor(`Register ${kind} revision`,[jsonField('Immutable revision','revision',initial)],async(f,status)=>{const r=await api(kind,{method:'POST',body:parse(f.revision)});status.textContent=`Retained ${r.id}@${r.version}`;},'Register revision')[1];
    const revisions=el('details',{},el('summary',{},'Dataset and evaluator versions'),revisionEditor('datasets',{id:'',version:1,document:{operation:'',cases:[{id:'case-01',text:'',attachments:[]}]}}),revisionEditor('evaluators',{id:'',version:1,document:{decision_rule:'',quality_rubric:''}}));
    return [heading('Benchmarks',link('New benchmark','/benchmarks/new')),panel(items(page).length?items(page).map(c=>el('div',{class:'row'},link(`${c.use_case} / ${c.id}`,`/benchmarks/${c.id}`),el('p',{class:'muted'},`${c.dataset_version} / ${c.evaluator_version}`))):note('No comparisons recorded.')),pageLink('/benchmarks',page),requireOwner(revisions)];
  }
  const comparison=await api(`comparisons/${id}`);
  const actions=el('div',{class:'actions'},button('Refresh',()=>render()));
  if(role()==='owner') actions.append(button('Resume fixed samples',async()=>{if(!confirm('Admit or resume the frozen sample matrix within its original limits? Existing sample identities are reused.'))return;try{await api(`comparisons/${id}/run`,{method:'POST'});render();}catch(e){alert(e.message);}}));
  const samples=comparison.samples || [];
  const sampleViews=samples.map(s=>{
    const outputs=s.technical?.manifest?.items || [];
    const score=editor('Record quality judgment',[input('Quality (0–100)','score','','number',{min:0,max:100,required:''}),field('Judgment','judgment','','textarea',{required:''}),input('Reviewer attribution (self-reported)','reviewer','', 'text',{required:''})],async(f,status)=>{await api(`comparisons/${id}/scores`,{method:'POST',body:{sample:s.sample,score:Number(f.score),judgment:f.judgment,reviewer:f.reviewer}});status.textContent='Judgment retained; refresh to inspect.';},'Record judgment',`quality:${s.sample}`)[1];
    return panel(el('h3',{},`${s.case || s.sample} / ${s.method} / repeat ${s.repeat}`),properties({'Job state':s.job?.phase,'Technical score':s.technical_score,'Human quality':s.human_quality?.score,'Credits':s.job?.usage?.credits,'Elapsed ms':s.duration_ms}),s.job?link('Original job',`/jobs/${s.job.id}`):note('Sample not admitted.'),outputs.map(o=>el('div',{class:'row'},link(`${o.content_type} / ${short(o.object_id)}`,`/assets/${o.object_id}`),o.content_type==='image/png'?preview({id:o.object_id,content_type:o.content_type}):el('p',{class:'muted'},'Preview unavailable for this type; open the asset for retained related renders and download.'))),s.human_quality?jsonDetails('Retained human judgment',s.human_quality):s.job?.phase==='succeeded'?el('details',{},el('summary',{},'Add quality judgment'),score):null);
  });
  const useCase=comparison.use_case || comparison.plan?.use_case;
  const recommendation=useCase ? await api('recommendations?'+new URLSearchParams({use_case:useCase,constraints:JSON.stringify(comparison.plan?.constraints || {}),...(comparison.plan?.operation?{operation:comparison.plan.operation}:{})})) : {qualified:null,reason:'No use case recorded.'};
  return [heading(useCase || `Benchmark ${short(id)}`,link('Benchmarks','/benchmarks')),actions,comparison.state==='incomplete'?note('Incomplete evidence. Missing, failed, unknown-usage and unjudged cases remain visible.'):null,
    panel(el('h2',{},'Frozen plan'),jsonDetails('Dataset, evaluator, methods and bounds',comparison.plan || comparison,true)),
    panel(el('h2',{},'Candidate evidence'),metricTable(comparison.method_metrics || []),jsonDetails('Full metrics',comparison.method_metrics || comparison.results),jsonDetails('Missing evidence',comparison.missing_cases || [])),
    el('h2',{},'Case outputs'),el('div',{class:'samples'},sampleViews),panel(el('h2',{},'Scoped recommendation'),el('p',{},recommendation.recommended ? value(recommendation.recommended) : `None qualified: ${recommendation.none_reason || recommendation.reason || 'No promoted route.'}`),properties({'Revision':recommendation.revision,'Observed':recommendation.observed_at}),jsonDetails('Scope, records and qualification evidence',recommendation)),recommendationForm(comparison,methodsFromComparison(comparison))];
}

function metricTable(metrics) {
  if(!metrics.length)return note('No comparable metrics recorded.');
  const names=['Pinned method','Outcomes','Human quality','Credits','Job time','Failures'];
  return el('table',{class:'metrics'},el('thead',{},el('tr',{},names.map(n=>el('th',{scope:'col'},n)))),el('tbody',{},metrics.map(m=>el('tr',{},[m.method,`${m.terminal} / ${m.samples} terminal`,`${value(m.human_score_mean)} / ${m.human_judgments_missing} missing`,value(m.credits),m.duration_ms==null?'Unknown':`${(m.duration_ms/1000).toFixed(2)} s`,`${m.failures} / ${m.samples}`].map((v,i)=>el('td',{},el('span',{class:'metric-label'},names[i]+': '),v))))));
}
function methodsFromComparison(c){return c.method_versions || [...new Set((c.samples||[]).map(s=>s.method))];}
function recommendationForm(c,pins){
  const form=editor('Propose scoped recommendation',[input('Pinned method','method',pins[0]||''),input('Evidence observed at (ISO UTC)','observed_at',new Date().toISOString()),jsonField('Exact constraints','constraints',c.plan?.constraints || {}),field('Evidence and limitations','evidence','','textarea',{required:''})],async(f,status)=>{const record=await api('recommendations',{method:'POST',body:AssetQRequest.recommendation(f,c)});status.replaceChildren('Proposal retained. ',requireOwner(button('Promote this exact scope',async()=>{if(!confirm('Promote this pinned method for the displayed use case and exact constraints?'))return;try{await api(`recommendations/${record.id}/promote`,{method:'POST'});status.textContent='Promotion recorded.';}catch(e){status.textContent=e.message;}})));},'Retain proposal')[1];
  return el('details',{},el('summary',{},'Recommendation proposal and owner promotion'),form);
}

async function render(restore = false) {
  rememberDraft();
  clearTimeout(pollTimer); const token = ++epoch;
  const params=new URLSearchParams(location.search), selectedProject=params.get('project');
  if(!session)return;
  if(selectedProject && project && project !== selectedProject) { lastView=null;drafts.clear();views.clear(); }
  if(selectedProject && !session.projects.some(p=>p.id===selectedProject)){main.replaceChildren(heading('Permission denied'),note('This URL names an unauthorized project. No data was requested.'),button('Open an authorized project',()=>navigate('/assets')));return;}
  project=selectedProject || session.projects[0]?.id;
  if(!project){loseSession('No project authority is available.');return;}
  if(!selectedProject){history.replaceState(history.state,'',url(location.pathname));}
  document.querySelector('#project').value=project;document.querySelector('#identity').textContent=`${role()} / scoped session`;
  document.querySelector('.brand').href=url('/assets');
  const parts=location.pathname.split('/').filter(Boolean),section=parts[0] || 'assets';
  if(parts.length<=1)views.set(`${project}:${section}`,location.pathname+location.search);
  const nav=document.querySelector('#nav');nav.replaceChildren(...[['assets','Assets'],['jobs','Jobs'],['methods','Methods'],['benchmarks','Benchmarks'],['operations','Operations']].map(([key,label])=>el('a',{href:views.get(`${project}:${key}`)||url('/'+key),'aria-current':key===section?'page':null},label)));
  main.replaceChildren(el('p',{role:'status',class:'busy'},'Loading scoped records…'));
  const viewKey=location.pathname+location.search;
  if(lastView?.key!==viewKey)lastView=null;
  try {
    let nodes;
    if(section==='assets' && parts.length<=2)nodes=await assets(params,token,parts[1]);
    else if(section==='jobs' && parts.length<=2)nodes=await jobs(params,token,parts[1]);
    else if(section==='methods' && (parts.length===1 || (parts.length===4 && parts[2]==='versions')))nodes=await methods(params,token,parts[1],parts[3]);
    else if(section==='benchmarks' && parts.length<=2)nodes=await benchmarks(params,token,parts[1]);
    else if(section==='operations' && parts.length===1){const status=await api('status');nodes=[heading('Operations',button('Refresh',()=>render())),note('Physical store and capacity values describe the service. Missing, stale and deferred backup/restore observations remain unknown.'),panel(jsonDetails('Scoped jobs, methods and recommendations',status,true))];}
    else nodes=[heading('Unknown destination'),note('This route is unsupported. The requested view was not restored.'),link('Asset library','/assets')];
    if(token!==epoch)return;main.replaceChildren(...nodes.filter(Boolean));
    lastView={key:viewKey,nodes:[...main.childNodes],observed:new Date().toISOString()};
    if(restore)requestAnimationFrame(()=>{const anchor=[...main.querySelectorAll('a')].find(a=>a.getAttribute('href')===history.state?.focus);(anchor || main).focus({preventScroll:true});scrollTo(0,history.state?.scroll || 0);});else{scrollTo(0,0);main.focus({preventScroll:true});}
  }catch(e){if(token===epoch){if(lastView?.key===viewKey)main.replaceChildren(note(`Last-known view is stale (read ${lastView.observed}): ${e.message}`),...lastView.nodes,button('Retry read',()=>render()));else main.replaceChildren(heading('Unable to load this view'),note(e.message),button('Retry read',()=>render()),link('Asset library','/assets'));}}
}
async function bootstrap(){
  try{
    const response=await fetch('/browser/session',{credentials:'same-origin',cache:'no-store'}),data=await response.json();
    if(!response.ok)throw new Error(data.reason || 'Browser identity is unavailable.');session=data;
    document.querySelector('#project').replaceChildren(...session.projects.map(p=>el('option',{value:p.id},p.id)));
    render();
  }catch(e){loseSession(e.message);}
}
document.querySelector('#project').addEventListener('change',e=>{drafts.clear();main.replaceChildren();navigate(url('/assets',{project:e.target.value}));});
document.querySelector('#logout').addEventListener('click',async()=>{try{const r=await fetch('/browser/logout',{method:'POST',credentials:'same-origin',headers:{'X-CSRF-Token':session?.csrf||''}});if(!r.ok)throw new Error('Could not confirm session termination.');loseSession('AssetQ session ended. Host sign-in is managed separately.');}catch(e){loseSession(e.message);}});
bootstrap();
