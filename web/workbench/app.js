
const SECRET = "imperium-v0-local-dev-secret";
const SUBJECT = "workbench-user";
const ECHO_CAP = "cap.echo";
const WRITE_CAP = "cap.write";
const GUEST_B64 = "AGFzbQEAAAABDgJgAn9/AGAEf39/fwF/AhoCBGhvc3QEZWNobwAABGhvc3QFd3JpdGUAAQMDAgABBQMBAAEHIQMGbWVtb3J5AgAIcnVuX2VjaG8AAglydW5fd3JpdGUAAwoXAggAIAAgARAACwwAIAAgASACIAMQAQs=";

const intents = new Map();
const scratch = new Map();
const seenNonces = new Set();
const revokedIds = new Set();

function nowIso(){ return new Date().toISOString(); }
function uid(){ return crypto.randomUUID(); }

function resolveScratchPath(raw){
  const trimmed = raw.trim();
  if(!trimmed) return {ok:false,error:"Path is empty."};
  const norm = trimmed.replace(/\\/g,"/");
  if(norm.startsWith("/")) return {ok:false,error:"Absolute paths are denied."};
  if(/^[a-zA-Z]:/.test(norm)) return {ok:false,error:"Drive-letter paths are denied."};
  const parts = norm.split("/").filter(p=>p.length>0);
  if(parts.some(p=>p===".."||p===".")) return {ok:false,error:"Path escape (..) is denied."};
  const rest = parts[0]==="scratch" ? parts.slice(1) : parts;
  if(rest.length===0) return {ok:false,error:"Path must name a file under scratch/."};
  return {ok:true, path:`scratch/${rest.join("/")}`};
}

function compileRules(nl){
  const source = nl.trim();
  if(!source) return {ok:false,error:"Natural language source is empty."};
  const echo = source.match(/^echo this message:\s*(.+)$/is);
  if(echo){
    const message = echo[1].trim();
    if(!message) return {ok:false,error:"Echo message is empty."};
    const ir = baseIr(`Echo ${message.slice(0,40)}`, source, message, ECHO_CAP, null);
    return {ok:true, ir};
  }
  const write = source.match(/^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is);
  if(write){
    const resolved = resolveScratchPath(write[1]);
    if(!resolved.ok) return resolved;
    if(!write[2].length) return {ok:false,error:"Write contents are empty."};
    const ir = baseIr(`Write ${resolved.path}`, source, write[2], WRITE_CAP, resolved.path);
    return {ok:true, ir};
  }
  return {ok:false,error:"v0 rules compiler only accepts: Echo this message: <text>  OR  Write file <path> with contents <text>"};
}

function baseIr(name, source, description, cap, target_path){
  return {
    id: uid(), name, nl_source: source,
    goal:{description:name, category:"Automation", priority:"Normal"},
    constraints:[], success_criteria:[],
    tasks:[{id:uid(), name:cap==ECHO_CAP?"Echo":"Write", description, kind:"Custom", capabilities:[cap], dependencies:[], estimated_duration_ms:10, target_path}],
    risk_score:0, requires_approval:true, version:1,
    compiled_at: nowIso(), compiler_version:"imperium-intent-rules-0.1.0"
  };
}

function localPropose(nl){
  const source = nl.trim();
  if(!source) return {ok:false,error:"Natural language source is empty."};
  if(/\b(rm\s|sudo|curl|wget|http:|https:|shell|bash|eval|network|download)\b/i.test(source))
    return {ok:false,error:"Proposal rejected: banned verb or network reference."};
  if(/^echo this message:\s*(.+)$/is.test(source) || /^write file\s+(.+?)\s+with contents\s+([\s\S]+)$/is.test(source))
    return {ok:true, canonical:source, proposer:"rules"};
  const echo = source.match(/^(?:please\s+)?(?:echo|say|print|repeat|tell me)\s*(?:this\s+message\s*)?[:\-]?\s+["']?(.+?)["']?$/is);
  if(echo?.[1]?.trim()) return {ok:true, canonical:`Echo this message: ${echo[1].trim()}`, proposer:"local"};
  const write = source.match(/^(?:please\s+)?(?:write|save|create|put)\s+(?:a\s+)?(?:file\s+)?(.+?)\s+(?:with(?:\s+contents)?|containing|as)\s+["']?([\s\S]+?)["']?$/is);
  if(write?.[1]?.trim() && write[2]!=null && write[2].length>0)
    return {ok:true, canonical:`Write file ${write[1].trim()} with contents ${write[2]}`, proposer:"local"};
  return {ok:false,error:"Local proposer could not map this to echo or write."};
}

function canonicalPayload(claims){
  return JSON.stringify({
    capability:claims.capability, expires_at:claims.expires_at, id:claims.id,
    intent_id:claims.intent_id, issued_at:claims.issued_at, nonce:claims.nonce,
    permissions:{env:[...claims.permissions.env].sort(), fs:[...claims.permissions.fs].sort(), net:[...claims.permissions.net].sort()},
    subject:claims.subject
  });
}
function toHex(buf){ return [...new Uint8Array(buf)].map(b=>b.toString(16).padStart(2,"0")).join(""); }
async function signClaims(claims, secret){
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(secret), {name:"HMAC", hash:"SHA-256"}, false, ["sign"]);
  const sig = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(canonicalPayload(claims)));
  return toHex(sig);
}
async function issueToken(input, secret, now=Date.now()){
  const claims = {
    id:input.id??uid(), capability:input.capability, subject:input.subject, intent_id:input.intent_id,
    permissions:{fs:[...input.permissions.fs], net:[...input.permissions.net], env:[...input.permissions.env]},
    nonce:input.nonce??uid(), issued_at:input.issued_at??now, expires_at:input.expires_at
  };
  return {...claims, signature: await signClaims(claims, secret)};
}
async function verifyToken(token, ctx){
  if(!token.signature) return {ok:false, reason:"empty signature"};
  if(await signClaims(token, ctx.secret) !== token.signature) return {ok:false, reason:"invalid signature"};
  if((ctx.now??Date.now()) >= token.expires_at) return {ok:false, reason:"expired"};
  if(ctx.revokedIds?.has(token.id)) return {ok:false, reason:"revoked"};
  if(ctx.seenNonces?.has(token.nonce)) return {ok:false, reason:"nonce reused"};
  if(![ECHO_CAP, WRITE_CAP].includes(token.capability)) return {ok:false, reason:"unknown capability"};
  if(token.permissions.net.length) return {ok:false, reason:"network denied"};
  return {ok:true, reason:"ok"};
}
function guestBytes(){
  const bin = atob(GUEST_B64); const out = new Uint8Array(bin.length);
  for(let i=0;i<bin.length;i++) out[i]=bin.charCodeAt(i); return out;
}
function writeUtf8(mem,ptr,text){ const b=new TextEncoder().encode(text); mem.set(b,ptr); return b.length; }
function readUtf8(mem,ptr,len){ return new TextDecoder().decode(mem.subarray(ptr,ptr+len)); }
async function runGuest(op, rights, effects){
  let echoOut="", memory;
  const imports={host:{
    echo:(ptr,len)=>{ if(!rights.echo) throw new Error("host.echo denied"); echoOut=readUtf8(new Uint8Array(memory.buffer),ptr,len); effects.onEcho(echoOut); },
    write:(pp,pl,bp,bl)=>{ if(!rights.write) throw new Error("host.write denied"); const mem=new Uint8Array(memory.buffer); const path=readUtf8(mem,pp,pl); const contents=readUtf8(mem,bp,bl); effects.onWrite(path,contents); return contents.length; }
  }};
  const result = await WebAssembly.instantiate(guestBytes(), imports);
  const instance = result.instance || result;
  memory = instance.exports.memory;
  const mem = new Uint8Array(memory.buffer);
  if(op.kind==="echo"){ const len=writeUtf8(mem,64,op.text); instance.exports.run_echo(64,len); return echoOut; }
  const pathLen=writeUtf8(mem,64,op.path); const bodyPtr=64+pathLen+8; const bodyLen=writeUtf8(mem,bodyPtr,op.contents);
  const n=instance.exports.run_write(64,pathLen,bodyPtr,bodyLen); return `wrote ${op.path} (${n} bytes)`;
}

function pushEvent(id, kind, payload={}){
  const s=intents.get(id); if(!s) return;
  s.events.push({id:uid(), intent_id:id, kind, payload, created_at:nowIso()});
  s.intent.updated_at=nowIso();
}
function listIntents(){ return [...intents.values()].map(s=>s.intent).sort((a,b)=>b.created_at.localeCompare(a.created_at)); }
function listEvents(id){ return intents.get(id)?.events??[]; }
function listScratch(){ return [...scratch.values()].sort((a,b)=>a.path.localeCompare(b.path)); }

function compile(nl, proposer){
  const result = compileRules(nl);
  if(!result.ok) return result;
  const ir=result.ir;
  const intent={id:ir.id,name:ir.name,nl_source:ir.nl_source,status:"compiled",ir,simulation:null,output:null,created_at:nowIso(),updated_at:nowIso(),token:null};
  intents.set(ir.id,{intent,events:[],rawToken:null});
  if(proposer) pushEvent(ir.id,"IntentProposed",{proposer,canonical:nl});
  pushEvent(ir.id,"IntentCompiled",{name:ir.name,capability:ir.tasks[0]?.capabilities[0]??""});
  return {ok:true,intent};
}
function propose(nl){
  const p=localPropose(nl); if(!p.ok) return p;
  const c=compile(p.canonical,p.proposer); if(!c.ok) return c;
  return {ok:true,intent:c.intent,canonical:p.canonical,proposer:p.proposer};
}
function simulate(id){
  const s=intents.get(id); if(!s) return {ok:false,error:"Intent not found."};
  const caps=s.intent.ir.tasks.flatMap(t=>t.capabilities);
  const sim={success_probability:1,risk:0,duration_ms:10,notes:caps.map(c=>`Capability ${c} allowed.`)};
  s.intent.simulation=sim; s.intent.status="simulated";
  pushEvent(id,"IntentSimulated",sim);
  return {ok:true,intent:s.intent};
}
async function approve(id){
  const s=intents.get(id); if(!s) return {ok:false,error:"Intent not found."};
  if(s.intent.status!=="simulated" && s.intent.status!=="approved") return {ok:false,error:"Simulate before Approve."};
  const capability=s.intent.ir.tasks[0]?.capabilities[0];
  const permissions = capability===WRITE_CAP ? {fs:["scratch"],net:[],env:[]} : {fs:[],net:[],env:[]};
  const token=await issueToken({capability,subject:SUBJECT,intent_id:id,permissions,expires_at:Date.now()+15*60*1000}, SECRET);
  s.rawToken=token;
  s.intent.token={id:token.id,capability,fingerprint:token.signature.slice(0,12),expires_at:new Date(token.expires_at).toISOString(),revoked:false,used:false,permissions};
  s.intent.status="approved";
  pushEvent(id,"IntentApproved",{});
  pushEvent(id,"TokenIssued",{token_id:token.id,fingerprint:s.intent.token.fingerprint,capability});
  return {ok:true,intent:s.intent};
}
async function execute(id){
  const s=intents.get(id); if(!s) return {ok:false,error:"Intent not found."};
  if(!s.rawToken||!s.intent.token) return {ok:false,error:"Approve first to issue a token."};
  if(s.intent.token.revoked) return {ok:false,error:"Token revoked."};
  if(s.intent.token.used) return {ok:false,error:"Token already spent."};
  const v=await verifyToken(s.rawToken,{secret:SECRET,seenNonces,revokedIds,expectedIntentId:id});
  if(!v.ok){ s.intent.status="failed"; pushEvent(id,"TaskFailed",{reason:v.reason}); return {ok:false,error:`Token verify failed: ${v.reason}`}; }
  seenNonces.add(s.rawToken.nonce);
  pushEvent(id,"TaskStarted",{capability:s.rawToken.capability});
  try{
    const task=s.intent.ir.tasks[0];
    let output;
    if(s.rawToken.capability===ECHO_CAP){
      output=await runGuest({kind:"echo",text:task.description},{echo:true,write:false},{onEcho:()=>{},onWrite:()=>{throw new Error("no")}});
    } else {
      output=await runGuest({kind:"write",path:task.target_path,contents:task.description},{echo:false,write:true},{
        onEcho:()=>{throw new Error("no")},
        onWrite:(path,contents)=>{ scratch.set(path,{path,contents,updated_at:nowIso()}); }
      });
    }
    s.intent.output=output; s.intent.status="executed"; s.intent.token={...s.intent.token,used:true};
    pushEvent(id,"TaskSucceeded",{output});
    return {ok:true,intent:s.intent};
  }catch(err){
    const reason=err instanceof Error?err.message:"execute failed";
    s.intent.status="failed"; pushEvent(id,"TaskFailed",{reason}); return {ok:false,error:reason};
  }
}
function revoke(id){
  const s=intents.get(id); if(!s?.rawToken||!s.intent.token) return {ok:false,error:"No token to revoke."};
  revokedIds.add(s.rawToken.id); s.intent.token={...s.intent.token,revoked:true};
  pushEvent(id,"TokenRevoked",{token_id:s.rawToken.id}); return {ok:true,intent:s.intent};
}
function foldEvents(events){
  const state={status:"compiled",output:null,proposer:null,fail_reason:null};
  for(const ev of events){
    const p=ev.payload||{};
    if(ev.kind==="IntentProposed"){ state.proposer=String(p.proposer||""); }
    if(ev.kind==="IntentSimulated") state.status="simulated";
    if(ev.kind==="IntentApproved") state.status="approved";
    if(ev.kind==="TaskSucceeded"){ state.status="executed"; state.output=String(p.output||""); }
    if(ev.kind==="TaskFailed"){ state.status="failed"; state.fail_reason=String(p.reason||"failed"); }
  }
  return state;
}
function replay(id){
  const s=intents.get(id); if(!s) return {ok:false,error:"Intent not found."};
  const folded=foldEvents(s.events);
  const matches = folded.status===s.intent.status && folded.output===s.intent.output;
  pushEvent(id,"IntentReplayed",{matches,folded_status:folded.status});
  return {ok:true,folded,matches_store:matches,event_count:s.events.length};
}

// UI
let selected=null, folded=null;
const $ = (id)=>document.getElementById(id);
function toast(msg){ const t=$("toast"); t.hidden=false; t.textContent=msg; setTimeout(()=>t.hidden=true,2200); }
function renderList(){
  const ul=$("intent-list"); ul.innerHTML="";
  for(const item of listIntents()){
    const li=document.createElement("li");
    const b=document.createElement("button");
    b.className = selected?.id===item.id ? "active" : "";
    b.innerHTML=`<span>${escapeHtml(item.name)}</span><span class="mono muted">${item.status}</span>`;
    b.onclick=()=>{ selected=item; folded=null; render(); };
    li.appendChild(b); ul.appendChild(li);
  }
  const su=$("scratch-list"); su.innerHTML="";
  const files=listScratch();
  if(!files.length){ su.innerHTML=`<li class="muted">Empty. Write file hello.txt with contents hi</li>`; }
  else for(const f of files){
    const li=document.createElement("li");
    li.className="panel";
    li.innerHTML=`<p class="mono">${escapeHtml(f.path)}</p><p class="muted">${escapeHtml(f.contents)}</p>`;
    su.appendChild(li);
  }
}
function escapeHtml(s){ return String(s).replace(/[&<>"']/g,c=>({"&":"&","<":"<",">":">","\"":""","'":"&#39;"}[c])); }
function renderDetail(){
  const el=$("detail");
  if(!selected){ el.innerHTML=`<p class="muted">Compile an echo or write intent. Execute is rejected until Approve.</p>`; return; }
  const order=["compiled","simulated","approved","executed"];
  const idx=Math.max(0, order.indexOf(selected.status==="failed"?"compiled":selected.status));
  const steps=["compile","simulate","approve","execute"].map((s,i)=>`<li class="${i<=idx?"on":""}">${s}</li>`).join("");
  const sim = selected.simulation ? `
    <div class="stats">
      <div class="stat"><dt>Success</dt><dd>${Math.round(selected.simulation.success_probability*100)}%</dd></div>
      <div class="stat"><dt>Risk</dt><dd>${selected.simulation.risk}</dd></div>
      <div class="stat"><dt>Capability</dt><dd>${escapeHtml(selected.ir.tasks[0]?.capabilities[0]||"—")}</dd></div>
      <div class="stat"><dt>Token</dt><dd>${selected.token? (selected.token.revoked?"revoked":selected.token.used?"spent":"live"):"—"}</dd></div>
    </div>` : `<p class="muted">No simulation yet. Run Simulate before Approve.</p>`;
  const out = selected.output!=null ? `<div class="panel"><p class="muted" style="margin:0 0 .35rem;font-size:.75rem">Capability output</p><p class="mono" style="margin:0">${escapeHtml(selected.output)}</p></div>` : "";
  const fold = folded ? `<div class="panel"><p class="muted" style="margin:0 0 .35rem;font-size:.75rem">Folded event log</p>
    <div class="stats"><div class="stat"><dt>Status</dt><dd>${escapeHtml(folded.status)}</dd></div>
    <div class="stat"><dt>Match</dt><dd>${folded.matches?"yes":"diverged"}</dd></div>
    <div class="stat"><dt>Events</dt><dd>${folded.events}</dd></div></div></div>` : "";
  const events = listEvents(selected.id).map(ev=>`<li><span class="mono">${escapeHtml(ev.kind)}</span><span class="muted">${new Date(ev.created_at).toLocaleTimeString()}</span></li>`).join("");
  el.innerHTML = `
    <div class="row"><div><h2>${escapeHtml(selected.name)}</h2><p class="mono muted" style="margin:.2rem 0 0">${selected.id}</p></div><span class="chip">${selected.status}</span></div>
    <ol class="steps">${steps}</ol>
    <div class="actions">
      <button class="secondary" id="a-sim">Simulate</button>
      <button class="secondary" id="a-appr">Approve</button>
      <button id="a-exec">Execute</button>
      <button class="ghost" id="a-rev">Revoke</button>
      <button class="ghost" id="a-rep">Replay log</button>
    </div>
    ${sim}${out}${fold}
    <details><summary>Intent IR</summary><pre>${escapeHtml(JSON.stringify(selected.ir,null,2))}</pre></details>
    <div><h3>Event log</h3><ol class="events">${events||"<li class=muted>No events.</li>"}</ol></div>`;
  $("a-sim").onclick=()=>{ const r=simulate(selected.id); if(!r.ok) return toast(r.error); selected=r.intent; toast("Simulated"); render(); };
  $("a-appr").onclick=async()=>{ const r=await approve(selected.id); if(!r.ok) return toast(r.error); selected=r.intent; toast("Approved"); render(); };
  $("a-exec").onclick=async()=>{ const r=await execute(selected.id); if(!r.ok) return toast(r.error); selected=r.intent; toast("Executed"); render(); };
  $("a-rev").onclick=()=>{ const r=revoke(selected.id); if(!r.ok) return toast(r.error); selected=r.intent; toast("Token revoked"); render(); };
  $("a-rep").onclick=()=>{ const r=replay(selected.id); if(!r.ok) return toast(r.error); folded={status:r.folded.status,output:r.folded.output,matches:r.matches_store,events:r.event_count}; renderDetail(); toast(r.matches_store?"Replay matches store":"Replay diverged"); };
}
function render(){ renderList(); if(selected){ selected=listIntents().find(i=>i.id===selected.id)||selected; } renderDetail(); }
$("btn-compile").onclick=()=>{ const r=compile($("nl").value); if(!r.ok) return toast(r.error); selected=r.intent; toast("Compiled"); render(); };
$("btn-propose").onclick=()=>{ const r=propose($("nl").value); if(!r.ok) return toast(r.error); $("nl").value=r.canonical; selected=r.intent; toast(`Proposed via ${r.proposer}`); render(); };
render();
