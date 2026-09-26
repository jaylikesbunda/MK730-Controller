const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

// Tauri 2 with withGlobalTauri exposes window.__TAURI__.invoke.
// window.__TAURI__.core does NOT exist — checking it first silently sent every
// call to the mock backend, which is why the app always said "not connected".
const invoke = async (cmd, args = {}) => {
  if (window.__TAURI__?.invoke) return window.__TAURI__.invoke(cmd, args);
  return mock(cmd, args);
};

// ---- local demo fallback (so UI works in browser preview) ----
let demoKeys = [];
let demoColors = {};
async function mock(cmd, args) {
  if (cmd === "get_keymap") {
    if (!demoKeys.length) {
      // minimal TKL fallback if backend absent
      demoKeys = Array.from({ length: 87 }, (_, i) => ({ id: i, label: "K" + (i + 1), row: Math.floor(i / 15), col: i % 15, w: 1, cls: "" }));
    }
    return demoKeys;
  }
  if (cmd === "get_status") return { connected: false, demo: true, firmware: "browser-demo", mode: 1, active_profile: 0, log_tail: ["browser preview: backend not attached"] };
  if (cmd === "list_devices") return [{ vid: 0x2516, pid: 0x0067, interface: 1, path: "demo://mk730", product: "MK730 (browser demo)", demo: true }];
  if (cmd === "debug_usb") return { hid_total: 0, hid_cm: [], rusb_cm: [], rusb_error: "", hint: "" };
  if (cmd === "get_profiles") return [0,1,2,3,4].map(i=>({id:i,name:"P"+(i+1),effect_id:4,params:{p1_speed:32,p2:0,p3:32,color1:{r:124,g:58,b:237},color2:{r:0,g:0,b:0},multilayer:0},colormap:[],brightness:100}));
  if (cmd === "get_macros") return [];
  return "demo-ok";
}

const state = { keys: [], colors: {}, profiles: [], macros: [], activeProfile: 0, selMacro: null, tool: "paint", fx: 7, fxLevel: 3, fxDir: 0 };

function hex(c){ const h=(n)=>n.toString(16).padStart(2,"0"); return `#${h(c[0])}${h(c[1])}${h(c[2])}`; }
function parseColor(s){ return [parseInt(s.slice(1,3),16),parseInt(s.slice(3,5),16),parseInt(s.slice(5,7),16)]; }
function dimmed(hexStr, pct){
  const [r,g,b]=parseColor(hexStr);
  return `rgb(${Math.round(r*pct/100)},${Math.round(g*pct/100)},${Math.round(b*pct/100)})`;
}

async function refreshStatus(){
  try{
    const st = await invoke("get_status");
    const pill = $("#conn-pill");
    const sideDot = $("#side-dot"), sideStatus = $("#side-status");
    if (st.demo) {
      pill.textContent = "Not connected";
      pill.className = "pill demo";
      $("#status").textContent = "Not connected — plug in your keyboard and choose Check again in Settings.";
      if(sideDot) sideDot.style.background = "#f59e0b";
      if(sideStatus) sideStatus.textContent = "Not connected";
    } else {
      pill.textContent = "Connected";
      pill.className = "pill live";
      $("#status").textContent = `Connected · Profile ${st.active_profile+1} · Ready`;
      if(sideDot) sideDot.style.background = "#22c55e";
      if(sideStatus) sideStatus.textContent = "MK730 · Ready";
    }
    const tail = (st.log_tail||[]).filter(Boolean);
    const el = $("#log");
    if (el) el.textContent = tail.length ? "Ready. Your changes apply automatically." : "Ready.";
  }catch(e){ $("#status").textContent = "Something went wrong starting up."; }
}

function keyColor(id){
  const pct = +($("#bright").value||100);
  const base = state.colors[id];
  if(!base) return "";
  const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(base);
  if(!m) return base;
  return `rgb(${Math.round(+m[1]*pct/100)},${Math.round(+m[2]*pct/100)},${Math.round(+m[3]*pct/100)})`;
}

async function loadKeys(){
  state.keys = await invoke("get_keymap");
  const kbd = $("#kbd"); kbd.innerHTML = "";
  const rows = {};
  state.keys.forEach(k=>{ (rows[k.row] ||= []).push(k); });
  Object.keys(rows).sort().forEach(r=>{
    const div = document.createElement("div"); div.className="krow";
    rows[r].sort((a,b)=>a.col-b.col).forEach(k=>{
      const b = document.createElement("button");
      b.className = "key "+(k.cls||""); b.textContent = k.label;
      b.style.flexGrow = k.w; b.dataset.led = k.id;
      b.style.background = keyColor(k.id);
      b.onclick = ()=>onKey(b, k.id);
      div.appendChild(b);
    });
    kbd.appendChild(div);
  });
  const bars = $("#bars"); bars.innerHTML="";
}

function onKey(btn, led){
  let [r,g,b]=parseColor($("#pick").value);
  if(state.tool==="erase"){ r=g=b=0; }
  if(state.tool==="fill"){
    state.keys.forEach(k=>{ state.colors[k.id]=`rgb(${r},${g},${b})`; });
    $$("#kbd .key").forEach(x=>{ x.style.background = dimmed($("#pick").value, +($("#bright").value||100)); });
    schedulePush();
    return;
  }
  state.colors[led]=`rgb(${r},${g},${b})`;
  btn.style.background = keyColor(led);
  schedulePush();
}

let pushTimer = null;
function schedulePush(){
  clearTimeout(pushTimer);
  pushTimer = setTimeout(pushFullMap, 160);
}
async function pushFullMap(){
  const arr = Array.from({length:255},()=>[0,0,0]);
  state.keys.forEach(k=>{
    const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(state.colors[k.id]||"");
    if(m && k.id<255) arr[k.id]=[+m[1],+m[2],+m[3]];
  });
  try{ await invoke("set_colormap",{colors:arr}); refreshStatus(); }catch(e){}
}

// sidebar nav
$$(".nav").forEach(t=>t.onclick=()=>{
  $$(".nav").forEach(x=>x.classList.remove("active"));
  $$(".panel").forEach(x=>x.classList.remove("active"));
  t.classList.add("active");
  $("#tab-"+t.dataset.tab).classList.add("active");
});
// legacy tabs (if present)
$$(".tab").forEach(t=>t.onclick=()=>{
  $$(".tab").forEach(x=>x.classList.remove("active"));
  $$(".panel").forEach(x=>x.classList.remove("active"));
  t.classList.add("active");
  $("#tab-"+t.dataset.tab).classList.add("active");
});

// window controls (backend commands always work; JS API fallback for dev)
$("#btn-min").onclick = async()=>{ try{ await invoke("win_minimize"); }catch(e){} };
$("#btn-max").onclick = async()=>{ try{ await invoke("win_toggle_maximize"); }catch(e){} };
$("#btn-close").onclick = async()=>{ try{ await invoke("win_close"); }catch(e){ window.close(); } };

// lighting actions
function repaintKeys(){
  $$("#kbd .key").forEach(x=>{
    const led = +x.dataset.led;
    x.style.background = keyColor(led);
  });
}
$("#btn-fill").onclick = async()=>{
  const [r,g,b]=parseColor($("#pick").value);
  state.keys.forEach(k=>state.colors[k.id]=`rgb(${r},${g},${b})`); repaintKeys();
  await invoke("set_full_color",{r,g,b}); refreshStatus();
};
$("#btn-clear").onclick = async()=>{ state.colors={}; repaintKeys(); await invoke("set_full_color",{r:0,g:0,b:0}); refreshStatus(); };
$("#btn-apply-map").onclick = async()=>{ await pushFullMap(); };
// brush tools
$$("#tool-seg .seg-btn").forEach(b=>b.onclick=()=>{
  $$("#tool-seg .seg-btn").forEach(x=>x.classList.remove("active"));
  b.classList.add("active"); state.tool=b.dataset.tool;
});
$$("#swatches") // placeholder
const SW=["#7c3aed","#06b6d4","#22c55e","#eab308","#f97316","#ef4444","#ec4899","#ffffff"];
const sw=$("#swatches"); SW.forEach(c=>{ const d=document.createElement("div"); d.className="sw"; d.style.background=c; d.onclick=()=>$("#pick").value=c; sw.appendChild(d); });
$("#bright").oninput = e=>{ $("#bright-v").textContent=e.target.value+"%"; repaintKeys(); schedulePush(); };
// lightbar groups (firmware LED values)
const BARS={Left:[1,2,3,4],Right:[141,142,143,144],Front:[13,20,27,34,41,55,62,76,90,104,111,118,125],Logo:[69]};
$$("[data-bar]").forEach(b=>b.onclick=()=>{
  const [r,g,bl]=parseColor($("#pick").value);
  (BARS[b.dataset.bar]||[]).forEach(led=>{ state.colors[led]=`rgb(${r},${g},${bl})`; });
  schedulePush(); refreshStatus();
});

// effects (V2 firmware values)
const FX=[[1,"Custom paint"],[4,"Steady"],[5,"Breathing"],[6,"Color cycle"],[7,"Wave"],[8,"Ripple"],[9,"Crosshair"],[10,"Rain"],[11,"Stars"],[21,"Snow"],[20,"Fireball"],[19,"Heartbeat"],[23,"Water ripple"],[16,"Reactive fade"],[22,"Circle spectrum"],[13,"Customized"],[24,"Off"]];
const NEEDS_TWO = new Set([8,9,10,11,21,20,19,23,16,5]);
const NEEDS_DIR = new Set([7,22]);
const NEEDS_COLOR = new Set([4,5,8,9,10,11,21,20,19,23,16,1]);
function renderFxGrid(){
  const g=$("#fx-grid"); if(!g) return; g.innerHTML="";
  FX.forEach(([v,l])=>{
    const d=document.createElement("div");
    d.className="fx"+(state.fx===v?" active":"");
    d.innerHTML=`<b>${l}</b><span></span>`;
    d.onclick=()=>{ state.fx=v; renderFxGrid(); syncFxOptions(); };
    g.appendChild(d);
  });
}
function syncFxOptions(){
  const nm = (FX.find(x=>x[0]===state.fx)||[])[1]||"";
  const nn=$("#fx-name"); if(nn) nn.textContent=nm;
  const show=(id,on)=>{ const e=$(id); if(e) e.style.display=on?"":"none"; };
  show("#fx-speed-row", state.fx!==4 && state.fx!==1 && state.fx!==24 && state.fx!==13);
  show("#fx-dir-row", NEEDS_DIR.has(state.fx));
  show("#fx-colors", NEEDS_COLOR.has(state.fx));
}
$$("#fx-speed-seg .seg-btn").forEach(b=>b.onclick=()=>{
  $$("#fx-speed-seg .seg-btn").forEach(x=>x.classList.remove("active"));
  b.classList.add("active"); state.fxLevel=+b.dataset.v;
});
$$("#fx-dir-seg .seg-btn").forEach(b=>b.onclick=()=>{
  $$("#fx-dir-seg .seg-btn").forEach(x=>x.classList.remove("active"));
  b.classList.add("active"); state.fxDir=+b.dataset.v;
});
state.fxLevel=3; state.fxDir=0;
$("#btn-fx-apply").onclick=async()=>{
  const eid=state.fx;
  const level=state.fxLevel||3, dir=state.fxDir||0;
  const brightness=+($("#fx-bright").value||255);
  const c1=parseColor($("#fx-c1").value), c2=parseColor($("#fx-c2").value);
  await invoke("set_effect_params",{p:{eid,level,brightness,dir,c1,c2}});
  refreshStatus();
};
renderFxGrid(); syncFxOptions();

// profiles
function fxName(id){ const f=FX.find(x=>x[0]===+id); return f?f[1]:"Custom"; }
async function loadProfiles(){
  state.profiles = await invoke("get_profiles");
  const pl=$("#plist"); pl.innerHTML="";
  state.profiles.forEach(p=>{
    const c=p.params?p.params.color1:{r:124,g:58,b:237};
    const d=document.createElement("div"); d.className="prof"+(p.id===state.activeProfile?" active":"");
    d.innerHTML=`<span class="dot" style="background:rgb(${c.r},${c.g},${c.b})"></span><strong>${p.name}</strong><span class="muted">${fxName(p.effect_id)}</span>`;
    const b=document.createElement("button"); b.className="btn"; b.textContent="Use";
    b.onclick=async()=>{ await invoke("set_active_profile",{id:p.id}); state.activeProfile=p.id; $("#prof-json").value=JSON.stringify(p,null,2); loadProfiles(); refreshStatus(); };
    d.appendChild(b); pl.appendChild(d);
  });
  if(state.profiles[0]) $("#prof-json").value=JSON.stringify(state.profiles[state.activeProfile]||state.profiles[0],null,2);
}
$("#btn-prof-refresh").onclick=loadProfiles;
$("#btn-save-fw").onclick=async()=>{ await invoke("save_profile_fw"); refreshStatus(); };
$("#btn-export").onclick=()=>{ const blob=new Blob([$("#prof-json").value],{type:"application/json"}); const a=document.createElement("a"); a.href=URL.createObjectURL(blob); a.download="mk730-profile.json"; a.click(); };
$("#file-import").onchange=e=>{ const f=e.target.files[0]; if(!f)return; const r=new FileReader(); r.onload=()=>$("#prof-json").value=r.result; r.readAsText(f); };
$("#btn-apply-profile").onclick=async()=>{ try{ const p=JSON.parse($("#prof-json").value); await invoke("apply_profile_to_device",{p}); await invoke("upsert_profile",{p}); loadProfiles(); refreshStatus(); }catch(err){ alert("That file could not be read."); } };

// macros
async function loadMacros(){
  state.macros = await invoke("get_macros");
  const ml=$("#mlist"); if(!ml) return; ml.innerHTML="";
  if(!state.macros.length) ml.innerHTML='<p class="muted">No macros yet. Create one to get started.</p>';
  state.macros.forEach(m=>{
    const d=document.createElement("div"); d.className="mac";
    d.innerHTML=`<strong>${m.trigger}</strong><span class="muted">${m.name} · ${m.events.length} steps</span>`;
    d.onclick=()=>{ state.selMacro=m.id; $("#m-name").value=m.name; $("#m-trigger").value=m.trigger; $("#m-repeat").value=m.repeat||0; $("#m-events").value=(m.events||[]).map(e=>`0x${e.hid.toString(16).padStart(2,"0")} ${e.pressed?"down":"up"} ${e.delay_ms}`).join("\n"); };
    ml.appendChild(d);
  });
}
$("#btn-m-new").onclick=()=>{ const t=($("#m-trigger").value||"F5").toUpperCase(); state.selMacro=null; $("#m-name").value="Macro "+t; $("#m-text").value=""; $("#m-events").value="0x04 down 0\n0x04 up 120"; };
function textToEvents(text){
  const ev=[]; let t=0;
  for(const ch of text){
    let hid=0, mod=0;
    if(ch>="a"&&ch<="z") hid=ch.charCodeAt(0)-93;
    else if(ch>="A"&&ch<="Z"){ hid=ch.charCodeAt(0)-61; mod=2; }
    else if(ch>="1"&&ch<="9") hid=ch.charCodeAt(0)-19;
    else if(ch==="0") hid=39;
    else if(ch===" ") hid=44;
    else if(ch==="\n") hid=40;
    else if(ch==="-") hid=45;
    else continue;
    ev.push({hid,pressed:true,delay_ms:t===0?0:60,modifier:mod});
    ev.push({hid,pressed:false,delay_ms:60,modifier:0});
    t++;
  }
  return ev;
}
const _bmt=$("#btn-m-type"); if(_bmt) _bmt.onclick=()=>{
  const ev=textToEvents($("#m-text").value||"");
  $("#m-events").value=ev.map(e=>`0x${e.hid.toString(16).padStart(2,"0")} ${e.pressed?"down":"up"} ${e.delay_ms}`).join("\n");
};
$("#btn-m-save").onclick=async()=>{
  const trigger=($("#m-trigger").value||"F5").toUpperCase();
  const lines=$("#m-events").value.split("\n").map(s=>s.trim()).filter(Boolean);
  const events=[];
  for(const ln of lines){
    const m=/0x([0-9a-fA-F]+)\s+(down|up|press)\s*(\d+)?/.exec(ln) || /^([a-z0-9])\s+(down|up)\s*(\d+)?/i.exec(ln);
    if(!m){ alert("Could not understand this line: "+ln); return; }
    let hid, pressed, delay_ms;
    if(ln.startsWith("0x")||ln.startsWith("0X")){ hid=parseInt(m[1],16); pressed=m[2].toLowerCase()!=="up"; delay_ms=+(m[3]||0); }
    else { hid=m[1].toLowerCase().charCodeAt(0)-87; pressed=m[2].toLowerCase()!=="up"; delay_ms=+(m[3]||0); }
    events.push({hid,pressed,delay_ms,modifier:0});
    if(m[2].toLowerCase()==="press"){ events.push({hid,pressed:false,delay_ms:80,modifier:0}); }
  }
  const ex = state.macros.find(x=>x.id===state.selMacro);
  const obj={ id: ex?.id || ("m_"+Date.now()), name: $("#m-name").value||("Macro "+trigger), trigger, events, repeat:+$("#m-repeat").value||0, fw_state:"local_only" };
  state.macros = await invoke("save_macro",{m:obj}); state.selMacro=obj.id; loadMacros(); refreshStatus();
};
$("#btn-m-del").onclick=async()=>{ if(!state.selMacro)return; state.macros=await invoke("delete_macro",{id:state.selMacro}); state.selMacro=null; loadMacros(); };

// settings
function renderDevs(ds){
  const box=$("#devs"); if(!box) return;
  const real = ds.filter(d=>!d.demo);
  if(!real.length){
    box.innerHTML=`<div class="prof"><strong>No keyboard found</strong><span class="muted">Check the cable and try again</span></div>`;
  } else {
    box.innerHTML=real.map(d=>`<div class="prof"><strong>${d.product||"Cooler Master keyboard"}</strong><span class="muted">Ready</span></div>`).join("");
  }
}
$("#btn-scan").onclick=async()=>{
  const ds=await invoke("list_devices");
  renderDevs(ds);
  refreshStatus();
};
let lastDebug = "";
async function showDebug(){
  try{
    const d = await invoke("debug_usb");
    lastDebug = JSON.stringify(d,null,2);
    const pre = $("#debug");
    pre.style.display = "block";
    const lines = (d.hid_all||[]).map(h=>`${h.vid}:${h.pid} ${h.product||"?"} (${h.mfr||"?"}) up=${h.up} if=${h.interface}`).join("\n");
    pre.textContent = `Devices seen: ${d.hid_total}, matching keyboard: ${d.hid_cm.length}\n` + lines.slice(0,3000);
  }catch(e){ lastDebug = String(e); }
}
const _bd = $("#btn-debug"); if(_bd) _bd.onclick = showDebug;
const _bc = $("#btn-copy-debug"); if(_bc) _bc.onclick = async()=>{ if(!lastDebug) await showDebug(); try{ await navigator.clipboard.writeText(lastDebug); }catch(e){} };

window.addEventListener("error", (e)=>{
  const s = $("#status");
  if(s) s.textContent = "Something didn't load — reopen the app. (" + (e.message||"error") + ")";
});

async function safe(fn, name){
  try{ await fn(); }
  catch(e){ console.warn(name, e); }
}

(async function boot(){
  $("#status").textContent = "Looking for your keyboard…";
  await safe(loadKeys, "keys");
  await safe(loadProfiles, "profiles");
  await safe(loadMacros, "macros");
  try{ renderDevs(await invoke("list_devices")); }catch(e){}
  refreshStatus();
  setInterval(refreshStatus, 5000);
})();
