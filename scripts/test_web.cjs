// R3/R5/R7/R9: forms must preserve exact API keys, immutable pins and limits.
// Component proof only: no browser, provider or production qualification.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const request = require('../web/requests.js');

// R4/R9: exercise actual handlers; this DOM fixture proves no geometry.
function browser() {
  const vm = require('node:vm');
  const {readFileSync} = require('node:fs');
  class Node {
    constructor(tag = '', text = '') { this.tag = tag; this.text = text; this.children = []; this.attrs = {}; this.events = {}; }
    append(...children) {
      for (const child of children) {
        if (child.parent) child.parent.children = child.parent.children.filter(n => n !== child);
        child.parent = this; this.children.push(child);
      }
    }
    replaceChildren(...children) { this.children = []; this.append(...children); }
    remove() { if (this.parent) this.parent.children = this.parent.children.filter(n => n !== this); this.parent = null; }
    after(node) {
      if (!this.parent) return;
      node.remove(); node.parent = this.parent;
      this.parent.children.splice(this.parent.children.indexOf(this) + 1, 0, node);
    }
    setAttribute(key, value) { this.attrs[key] = value; }
    addEventListener(name, handler) { this.events[name] = handler; }
    getBoundingClientRect() { return this.rect || {height: this.height}; }
    contains(node) { return this === node || this.children.some(child => child.contains(node)); }
    get textContent() { return this.text + this.children.map(n => n.textContent).join(''); }
    set textContent(text) { this.text = text; this.children = []; }
    get childNodes() { return this.children; }
    get elements() { return []; }
    querySelectorAll() { return []; }
    focus() {}
  }
  const controls = new Map();
  const styles = new Map();
  const events = {}, frames = [], scrolls = [];
  let resize, observed;
  const context = vm.createContext({Node, URL, URLSearchParams, PipeRequest: request,
    location: {origin: 'https://pipe.ufobe.com', search: ''},
    document: {documentElement: {style: {setProperty: (key, value) => styles.set(key, value)}},
      createElement: tag => new Node(tag), createTextNode: text => new Node('', text),
      querySelector: selector => { if (!controls.has(selector)) controls.set(selector, new Node()); return controls.get(selector); },
      addEventListener: (name, handler) => { events[name] = handler; }}, addEventListener() {}, clearTimeout() {}, scrollTo() {},
    requestAnimationFrame: callback => frames.push(callback),
    scrollBy: (x, y) => scrolls.push([x, y]), innerHeight: 800,
    getComputedStyle: () => ({position: context.headerPosition || 'sticky'}),
    // Keep automatic sign-in pending; the fixture supplies an already scoped session.
    fetch: () => new Promise(() => {}),
    ResizeObserver: class { constructor(callback) { resize = callback; } observe(node) { observed = node; } }});
  vm.runInContext(readFileSync(require.resolve('../web/app.js'), 'utf8'), context);
  // R9: wrapping/text scaling must move the fixed rail on growth and shrinkage.
  assert.equal(observed, controls.get('header'));
  for (const height of [76, 217.35, 113.35, 76]) {
    observed.height = height; resize();
    assert.equal(styles.get('--header-height'), `${height}px`);
  }
  // R9/F3: native Tab/Shift+Tab must reveal controls without changing history restoration.
  assert.equal(typeof events.keydown, 'function');
  events.keydown({key: 'ArrowUp'}); events.keydown({key: 'Tab', defaultPrevented: true});
  assert.equal(frames.length, 0);
  const focused = new Node('input'); controls.get('#main').append(focused);
  observed.rect = {bottom: 217.35};
  for (const [top, bottom, expected] of [[200, 244, -23.35], [780, 840, 46], [230, 280, 0], [300, 1300, 76.65]]) {
    for (const shiftKey of [false, true]) {
      context.document.activeElement = new Node('button');
      events.keydown({key: 'Tab', shiftKey});
      // Default browser focus movement happens after keydown, before the next frame.
      context.document.activeElement = focused; focused.rect = {top, bottom};
      frames.shift()();
      if (expected) assert.ok(Math.abs(scrolls.pop()[1] - expected) < .001);
      else assert.equal(scrolls.length, 0);
    }
  }
  context.headerPosition = 'relative'; focused.rect = {top: -19, bottom: 20};
  events.keydown({key: 'Tab', shiftKey: true}); frames.shift()();
  assert.deepEqual(scrolls.pop(), [0, -25]);
  context.document.activeElement = new Node('button');
  events.keydown({key: 'Tab'}); frames.shift()();
  assert.equal(scrolls.length, 0);
  vm.runInContext("project = 'project-a'; session = {projects:[{id:project,role:'member'}]}; api = async path => path.includes('/related?') ? {parents:[],outputs:[],children:[]} : fixture;", context);
  return context;
}
const all = node => [node, ...node.children.flatMap(all)];
test('retained-render captions require a loaded model preview and disappear on failure', async () => {
  const context = browser(), vm = require('node:vm');
  const caption = 'Retained render of this job; not a live 3D view.';
  for (const type of ['model/gltf-binary', 'image/png', 'application/x-assetq-synthetic']) {
    context.fixture = {id:'isolated-copy', content_type:type, size:123, provenance:{name:'Invented isolated copy'}};
    const detail = await vm.runInContext("assetDetail(fixture.id, epoch)", context);
    assert.equal(detail.textContent.includes(caption), false, `${type}: loading/unavailable is not a retained render`);
    const original = all(detail).find(n => n.tag === 'a' && n.textContent === 'Download original');
    assert.equal(original.attrs.href, '/browser/v1/objects/isolated-copy/content?project=project-a');
    const image = all(detail).find(n => n.tag === 'img');
    if (!image) { assert.ok(detail.textContent.includes('Preview unavailable; download the original.')); continue; }
    assert.equal(image.attrs.src, '/browser/v1/objects/isolated-copy/preview?project=project-a');
    // Missing retained relationship or denied/corrupt preview must keep the fallback truthful.
    image.events.error();
    assert.ok(detail.textContent.includes(type));
    assert.ok(detail.textContent.includes('Preview unavailable; download the original.'));
    assert.equal(detail.textContent.includes(caption), false);
    const loaded = await vm.runInContext("assetDetail(fixture.id, epoch)", context);
    const loadedImage = all(loaded).find(n => n.tag === 'img');
    loadedImage.events.load?.(); loadedImage.events.load?.();
    assert.equal(all(loaded).filter(n => n.tag === 'small' && n.textContent === caption).length, type === 'model/gltf-binary' ? 1 : 0);
    loadedImage.events.error();
    assert.equal(loaded.textContent.includes(caption), false);
  }
  context.fixture.content_type = 'model/gltf-binary';
  const thumbnail = vm.runInContext('preview(fixture, true)', context);
  thumbnail.children[0].events.load?.();
  assert.equal(thumbnail.textContent.includes(caption), false);
  thumbnail.children[0].events.error();
  assert.equal(thumbnail.textContent, 'File');
});

// A4/R4/specq Time: evidence panels must not leave timestamps only in raw JSON.
test('history, qualification and operations handlers render nested evidence times', async t => {
  for (const surface of ['history', 'qualification', 'operations']) await t.test(surface, async () => {
    const context = browser(), vm = require('node:vm');
    context.now = new Date(2026, 10, 5, 14, 30).getTime();
    context.Date = class extends Date { static now() { return context.now; } };
    context.location.pathname = '/operations'; context.location.search = '?project=project-a';
    const read = expression => vm.runInContext(expression, context);
    const expressions = {
      history: "jobs(new URLSearchParams(), epoch, 'invented-job')",
      qualification: "methods(new URLSearchParams(), epoch, 'invented-method', 1)",
      operations: 'render().then(() => main.childNodes)'
    };
    for (const timestamp of ['2026-11-05 12:00:00+00', null, 'invalid']) {
      context.timestamp = timestamp;
      read(`api = async path => path.endsWith('/events') ? [{seq:1,event:{type:'completed'},at:timestamp}] :
        path.startsWith('jobs/') ? {phase:'succeeded'} : path.startsWith('methods/') ?
        {id:'invented-method',version:1,spec:{},qualification:{observed_at:timestamp}} :
        {providers:[{value:{health:'unknown'},observed_at:timestamp}],backup:{value:{state:'unknown'},observed_at:null}};`);
      const nodes = await read(expressions[surface]);
      const title = {history:'Durable event history',qualification:'Qualification and limitations',operations:'Scoped jobs, methods and recommendations'}[surface];
      const detail = nodes.filter(Boolean).flatMap(all).find(n => n.tag === 'details' && n.children[0].textContent === title);
      assert.ok(detail, `${surface}: evidence panel exists`);
      const times = all(detail).filter(n => n.tag === 'time');
      if (timestamp === null || timestamp === 'invalid') {
        assert.equal(times.length, 0); assert.ok(detail.textContent.includes('? unavailable'));
      } else {
        assert.equal(times.length, 1, `${surface}: timestamp needs a human display`);
        assert.equal(times[0].textContent, read('age(timestamp, now)'));
        assert.equal(times[0].attrs.title, read('exactTime(timestamp, now)'));
        assert.equal(times[0].attrs.datetime, new Date(timestamp).toISOString());
      }
      assert.ok(all(detail).find(n => n.tag === 'pre').textContent.includes(JSON.stringify(timestamp)), 'original evidence stays intact');
    }
  });
});

test('default and cleared filters omit exact empty predicates', () => {
  for(const path of ['jobs?phase=&after=&limit=50','methods?operation=&after=&limit=100']) {
    const url=request.apiUrl(path,'project-a','https://pipe.ufobe.com');
    assert.equal(url.searchParams.has('phase'),false);assert.equal(url.searchParams.has('operation'),false);
    assert.equal(url.searchParams.get('project'),'project-a');assert.ok(url.searchParams.get('limit'));
  }
  assert.equal(request.apiUrl('jobs?phase=failed','project-a','https://pipe.ufobe.com').searchParams.get('phase'),'failed');
});
test('unknown submissions retain the original key; verified recovery permits another request', async () => {
  const data=new Map(), storage={getItem:k=>data.get(k)||null,setItem:(k,v)=>data.set(k,v),removeItem:k=>data.delete(k)};
  let seq=0;const make=()=>`key-${++seq}`, absent=async()=>{throw {status:404};};
  const first=await request.pendingRequest(storage,'pending',absent,make);assert.equal(first.key,'key-1');
  assert.equal((await request.pendingRequest(storage,'pending',absent,make)).key,first.key);
  await assert.rejects(request.pendingRequest(storage,'pending',async()=>{throw new Error('observation unknown');},make));
  assert.equal(storage.getItem('pending'),first.key);
  const recovered=await request.pendingRequest(storage,'pending',async key=>({id:'original-job',key}),make);
  assert.equal(recovered.job.id,'original-job');assert.equal(storage.getItem('pending'),null);
  assert.equal((await request.pendingRequest(storage,'pending',absent,make)).key,'key-2');
});

test('generation preserves the retained key and pin without accepting authority fields', () => {
  const method={id:'image.worker.v1',version:5,spec:{operation:'image.generate'}};
  const form={method:'image.worker.v1@5',text:'original brief',attachments:'image-1\nimage-2',constraints:'{"max_width":1024}',max_credits:'0',permissions:'invented owner'};
  const body=request.job(form,method,'retained-key');
  assert.deepEqual(body,{method:form.method,operation:'image.generate',text:'original brief',attachments:['image-1','image-2'],constraints:{max_width:1024},budget:{max_credits:0},idempotency_key:'retained-key'});
  assert.deepEqual(request.job(form,method,'retained-key'),body);
  assert.throws(()=>request.job({...form,method:'image.worker.v1@6'},method,'retained-key'));
  for(const max_credits of ['','-1','NaN','1.5','9007199254740992']) assert.throws(()=>request.job({...form,max_credits},method,'retained-key'));
  assert.throws(()=>request.job({...form,attachments:Array(17).fill('id').join('\n')},method,'retained-key'));
  for(const constraints of ['null','[]','"text"','{']) assert.throws(()=>request.job({...form,constraints},method,'retained-key'));
});
test('benchmark and method forms use the published versioned API shapes', () => {
  const f={use_case:'props',dataset:'inputs@2',evaluator:'quality@3',constraints:'{}',repeats:'2',max_jobs:'4',max_credits:'0'};
  assert.deepEqual(request.comparison(f,['synthetic.fake@1']),{use_case:'props',dataset_version:'inputs@2',evaluator_version:'quality@3',method_versions:['synthetic.fake@1'],plan:{dataset:'inputs@2',evaluator:'quality@3',constraints:{},repeats:2,max_jobs:4,max_credits:0}});
  assert.throws(()=>request.comparison({...f,max_jobs:'33'},['synthetic.fake@1']));
  assert.throws(()=>request.comparison(f,['synthetic.fake@01']));
  assert.deepEqual(request.method({id:'ignored',status:'disabled',spec:'{"model":"retained"}'},'original'),{id:'original',status:'disabled',spec:{model:'retained'}});
  const c={id:'comparison-1',plan:{use_case:'props'}};
  const r=request.recommendation({method:'synthetic.fake@1',observed_at:'2026-11-05T00:00:00Z',constraints:'{}',evidence:'invented judgment'},c);
  assert.equal(r.use_case,'props');assert.equal(r.evidence.comparison,c.id);assert.equal(r.observed_at,'2026-11-05T00:00:00Z');
});

// A4/specq Time: boundary rounding and missing observations must stay truthful.
test('human times round at minute, five-hour and day boundaries with exact local hover', async () => {
  const context = browser(), vm = require('node:vm');
  context.now = new Date(2026, 10, 5, 14, 30).getTime();
  const read = expression => vm.runInContext(expression, context);
  for (const [delta, expected] of [[-1,'<1m ago'],[0,'<1m ago'],[59999,'<1m ago'],[60000,'1m ago'],[3599999,'59m ago'],[3600000,'1h ago'],[86399999,'23h ago'],[86400000,'1d ago']]) {
    context.time = context.now - delta;
    assert.equal(read('age(time, now)'), expected);
  }
  for (const [delta, expected] of [[-1,'now'],[0,'now'],[1,'0:01'],[60000,'0:01'],[60001,'0:02'],[17999999,'5:00'],[18000000,'5h'],[18000001,'6h'],[86399999,'24h'],[86400000,'1d'],[86400001,'2d']]) {
    context.time = context.now + delta;
    assert.equal(read('countdown(time, now)'), expected);
  }
  for (const missing of [null, undefined, '', 'invalid', NaN, Infinity]) {
    context.time = missing;
    for (const call of ['age(time, now)','countdown(time, now)','exactTime(time, now)','timeView(time)']) assert.equal(read(call), '? unavailable');
  }
  context.time = context.now;
  assert.equal(read('exactTime(time, now)'), 'Thu Nov 05, 14:30');
  context.time = new Date(2025, 10, 5, 14, 30).getTime();
  assert.equal(read('exactTime(time, now)'), 'Wed Nov 05 2025, 14:30');
  context.time = context.now - 60000;
  const node = read('timeView(time, false, now)');
  assert.equal(node.tag, 'time'); assert.equal(node.textContent, '1m ago');
  assert.equal(node.attrs.title, 'Thu Nov 05, 14:29');
  assert.equal(node.attrs.datetime, new Date(context.time).toISOString());
  assert.equal(read('timeView(now + 1, true, now)').textContent, '0:01');
  assert.equal(read('age(0, 60000)'), '1m ago');
  context.Date = class extends Date { static now() { return context.now; } };
  context.fixture = {id:'invented-object', content_type:'image/png', size:1,
    created_at:new Date(context.now - 60000).toISOString(), retention_until:new Date(context.now + 1).toISOString()};
  const detail = await read('assetDetail(fixture.id, epoch)');
  assert.deepEqual(all(detail).filter(n => n.tag === 'time').map(n => n.textContent), ['1m ago','0:01']);
  context.fixture.retention_until = 'invalid';
  assert.ok((await read('assetDetail(fixture.id, epoch)')).textContent.includes('? unavailable'));
  assert.equal((await read('assetDetail(fixture.id, epoch)')).textContent.includes('No automatic deletion'), false);
  context.fixture.retention_until = null;
  assert.ok((await read('assetDetail(fixture.id, epoch)')).textContent.includes('No automatic deletion'));
  context.fixture = {id:'invented-job', phase:'succeeded', updated_at:new Date(context.now - 3600000).toISOString()};
  read("api = async path => path.endsWith('/events') ? [] : path.endsWith('/result') ? {} : fixture");
  for (const expression of ["jobs(new URLSearchParams(), epoch, fixture.id)", "(api = async () => [fixture], jobs(new URLSearchParams(), epoch))"]) {
    const nodes = await read(expression);
    const times = nodes.filter(Boolean).flatMap(all).filter(n => n.tag === 'time');
    assert.equal(times.length, 1); assert.equal(times[0].textContent, '1h ago');
    assert.equal(times[0].attrs.datetime, context.fixture.updated_at);
  }
});
