// R3/R5/R7/R9: forms must preserve exact API keys, immutable pins and limits.
// Component proof only: no browser, provider or production qualification.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const request = require('../web/requests.js');

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
