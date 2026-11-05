// R3/R5/R7/R9: forms must preserve exact API keys, immutable pins and limits.
// Component proof only: no browser, provider or production qualification.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const request = require('../web/requests.js');

// R4/R9: exercise actual detail/preview events; this DOM fixture proves no geometry.
test('header reflow stays visible and retained-render captions require a loaded model preview', async () => {
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
    getBoundingClientRect() { return {height: this.height}; }
    get textContent() { return this.text + this.children.map(n => n.textContent).join(''); }
  }
  const controls = new Map();
  const styles = new Map();
  let resize, observed;
  const context = vm.createContext({Node, URL, URLSearchParams, AssetQRequest: request,
    location: {origin: 'https://assets.ufobe.com', search: ''},
    document: {documentElement: {style: {setProperty: (key, value) => styles.set(key, value)}},
      createElement: tag => new Node(tag), createTextNode: text => new Node('', text),
      querySelector: selector => { if (!controls.has(selector)) controls.set(selector, new Node()); return controls.get(selector); },
      addEventListener() {}}, addEventListener() {},
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
  vm.runInContext("project = 'project-a'; session = {projects:[{id:project,role:'member'}]}; api = async path => path.includes('/related?') ? {parents:[],outputs:[],children:[]} : fixture;", context);
  const all = node => [node, ...node.children.flatMap(all)];
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

test('default and cleared filters omit exact empty predicates', () => {
  for(const path of ['jobs?phase=&after=&limit=50','methods?operation=&after=&limit=100']) {
    const url=request.apiUrl(path,'project-a','https://assets.ufobe.com');
    assert.equal(url.searchParams.has('phase'),false);assert.equal(url.searchParams.has('operation'),false);
    assert.equal(url.searchParams.get('project'),'project-a');assert.ok(url.searchParams.get('limit'));
  }
  assert.equal(request.apiUrl('jobs?phase=failed','project-a','https://assets.ufobe.com').searchParams.get('phase'),'failed');
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
