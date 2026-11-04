'use strict';
// Form-to-API payloads; the service remains the authority for every limit.
const AssetQRequest = (() => {
  function integer(v, label, min = 0, max = Number.MAX_SAFE_INTEGER) {
    const n = Number(v);
    if (String(v).trim() === '' || !Number.isSafeInteger(n) || n < min || n > max) throw new Error(`${label} must be an integer in ${min}–${max}.`);
    return n;
  }
  function object(s) {
    const v = typeof s === 'string' ? JSON.parse(s || '{}') : s;
    if (!v || Array.isArray(v) || typeof v !== 'object') throw new Error('Constraints must be an object.');
    return v;
  }
  function job(f, method, key) {
    if (!method || f.method !== `${method.id}@${method.version}`) throw new Error('Select the exact retained method version.');
    const attachments = f.attachments.trim() ? f.attachments.trim().split(/\s+/) : [];
    if (attachments.length > 16) throw new Error('At most 16 attachments per request.');
    return {method:f.method,operation:method.spec.operation,text:f.text,attachments,
      constraints:object(f.constraints),budget:{max_credits:integer(f.max_credits,'Maximum credits')},idempotency_key:key};
  }
  function comparison(f, pins) {
    if (!pins.length || pins.some(m => !/^.+@[1-9][0-9]*$/.test(m))) throw new Error('Select pinned candidate versions.');
    return {use_case:f.use_case,dataset_version:f.dataset,evaluator_version:f.evaluator,method_versions:pins,
      plan:{dataset:f.dataset,evaluator:f.evaluator,constraints:object(f.constraints),repeats:integer(f.repeats,'Repeats',1,8),
        max_jobs:integer(f.max_jobs,'Maximum jobs',1,32),max_credits:integer(f.max_credits,'Maximum credits')}};
  }
  function method(f, id) { return {id:id || f.id,status:f.status,spec:object(f.spec)}; }
  function recommendation(f, c) {
    if (!Number.isFinite(Date.parse(f.observed_at))) throw new Error('Supply the evidence observation time.');
    return {use_case:c.use_case || c.plan?.use_case,method:f.method,observed_at:f.observed_at,
      constraints:object(f.constraints),evidence:{comparison:c.id,judgment:f.evidence}};
  }
  return {job,comparison,method,recommendation};
})();
if (typeof module !== 'undefined') module.exports = AssetQRequest;
