const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];
const invoke = async (cmd, args = {}) => {
  if (window.__TAURI__?.core?.invoke) return window.__TAURI__.core.invoke(cmd, args);
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
  if (cmd === "list_devices") return [{ vid: 0x2512, pid: 0x0067, interface: 1, path: "demo://mk730", product: "MK730 (browser demo)", demo: true }];
  if (cmd === "get_profiles") return [0,1,2,3,4].map(i=>({id:i,name:"P"+(i+1),effect_id:4,params:{p1_speed:32,p2:0,p3:32,color1:{r:124,g:58,b:237},color2:{r:0,g:0,b:0},multilayer:0},colormap:[],brightness:100}));
  if (cmd === "get_macros") return [];
  return "demo-ok";
}

const state = { keys: [], colors: {}, profiles: [], macros: [], activeProfile: 0, selMacro: null };

function hex(c){ const h=(n)=>n.toString(16).padStart(2,"0"); return `#${h(c[0])}${h(c[1])}${h(c[2])}`; }
function parseColor(s){ return [parseInt(s.slice(1,3),16),parseInt(s.slice(3,5),16),parseInt(s.slice(5,7),16)]; }

async function refreshStatus(){
  try{
    const st = await invoke("get_status");
    $("#status").textContent = `${st.demo ? "demo" : "connected"} · fw ${st.firmware} · mode 41 0${st.mode} · P${st.active_profile+1}`;
    const pill = $("#conn-pill");
    pill.textContent = st.demo ? "demo" : "live";
    pill.className = "pill " + (st.demo ? "demo" : "live");
    $("#log").textContent = (st.log_tail||[]).join("\n");
  }catch(e){ $("#status").textContent = "status err "+e; }
}

async function loadKeys(){
  state.keys = await invoke("get_keymap");
  const kbd = $("#kbd"); kbd.innerHTML = "";
  const rows = {};
  state.keys.filter(k=>k.id<200).forEach(k=>{ (rows[k.row] ||= []).push(k); });
  Object.keys(rows).sort().forEach(r=>{
    const div = document.createElement("div"); div.className="krow";
    rows[r].sort((a,b)=>a.col-b.col).forEach(k=>{
      const b = document.createElement("button");
      b.className = "key "+(k.cls||""); b.textContent = k.label;
      b.style.flexGrow = k.w; b.title = `id ${k.id}`;
      b.style.background = state.colors[k.id] || "";
      b.onclick = ()=>{ const [rr,gg,bb]=parseColor($("#pick").value); paint(k.id,rr,gg,bb); };
      div.appendChild(b);
    });
    kbd.appendChild(div);
  });
  const bars = $("#bars"); bars.innerHTML="";
  const brow=document.createElement("div"); brow.className="krow";
  state.keys.filter(k=>k.id>=200).forEach(k=>{
    const b=document.createElement("button"); b.className="key bar"; b.textContent=k.label;
    b.onclick=()=>{ const [rr,gg,bb]=parseColor($("#pick").value); paint(k.id,rr,gg,bb); };
    brow.appendChild(b);
  });
  bars.appendChild(brow);
}

function paint(id,r,g,b){
  state.colors[id]=`rgb(${r},${g},${b})`;
  invoke("set_key_color",{id,r,g,b}).then(refreshStatus).catch(()=>{});
  loadKeysKeep();
}
function loadKeysKeep(){
  $$("#kbd .key").forEach(()=>{});
  loadKeys();
}

// tabs
$$(".tab").forEach(t=>t.onclick=()=>{
  $$(".tab").forEach(x=>x.classList.remove("active"));
  $$(".panel").forEach(x=>x.classList.remove("active"));
  t.classList.add("active");
  $("#tab-"+t.dataset.tab).classList.add("active");
});

// window controls
async function win(){ 
  if(window.__TAURI__?.window?.getCurrentWindow) return window.__TAURI__.window.getCurrentWindow();
  return null;
}
$("#btn-min").onclick = async()=>{ const w=await win(); w?w.minimize():null; };
$("#btn-max").onclick = async()=>{ const w=await win(); w?w.toggleMaximize():null; };
$("#btn-close").onclick = async()=>{ const w=await win(); w?w.close():window.close(); };

// lighting actions
$("#btn-fill").onclick = async()=>{
  const [r,g,b]=parseColor($("#pick").value);
  await invoke("set_full_color",{r,g,b}); refreshStatus();
  state.colors={}; state.keys.forEach(k=>state.colors[k.id]=`rgb(${r},${g},${b})`); loadKeys();
};
$("#btn-clear").onclick = async()=>{ await invoke("set_full_color",{r:0,g:0,b:0}); state.colors={}; loadKeys(); refreshStatus(); };
$("#btn-apply-map").onclick = async()=>{
  const arr = state.keys.filter(k=>k.id<128).map(k=>{
    const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(state.colors[k.id]||"rgb(0,0,0)");
    return m?[+m[1],+m[2],+m[3]]:[0,0,0];
  });
  while(arr.length<128) arr.push([0,0,0]);
  await invoke("set_colormap",{colors:arr}); refreshStatus();
};
$$("#swatches") // placeholder
const SW=["#7c3aed","#06b6d4","#22c55e","#eab308","#f97316","#ef4444","#ec4899","#ffffff","#64748b","#000000","#3b82f6","#a3e635","#facc15","#14b8a6","#f43f5e","#8b5cf6"];
const sw=$("#swatches"); SW.forEach(c=>{ const d=document.createElement("div"); d.className="sw"; d.style.background=c; d.onclick=()=>$("#pick").value=c; sw.appendChild(d); });
$$("[data-c]").forEach(b=>b.onclick=()=>{ $("#pick").value=b.dataset.c; });
$("#bright").oninput = e=>$("#bright-v").textContent=e.target.value+"%";

// effects
const FX=[[0,"Fully lit"],[1,"Breathe"],[2,"Color cycle"],[3,"Single key"],[4,"Wave"],[5,"Ripple"],[6,"Cross"],[7,"Raindrops"],[8,"Stars"],[9,"Snake"],[10,"Customized"],[224,"Multilayer"],[254,"Off"]];
const fxSel=$("#fx"); FX.forEach(([v,l])=>{ const o=document.createElement("option"); o.value=v; o.textContent=l; if(v===4)o.selected=true; fxSel.appendChild(o); });
$("#fx-p1").oninput=e=>$("#fx-p1-v").textContent=e.target.value;
$("#fx-p3").oninput=e=>$("#fx-p3-v").textContent=e.target.value;
$("#btn-fx-apply").onclick=async()=>{
  const eid=+fxSel.value, p1=+$("#fx-p1").value, p2=+$("#fx-p2").value, p3=+$("#fx-p3").value;
  const c1=parseColor($("#fx-c1").value), c2=parseColor($("#fx-c2").value);
  await invoke("set_effect",{eid});
  await invoke("set_effect_params",{p:{eid,p1,p2,p3,c1,c2}});
  refreshStatus();
};

// profiles
async function loadProfiles(){
  state.profiles = await invoke("get_profiles");
  const pl=$("#plist"); pl.innerHTML="";
  state.profiles.forEach(p=>{
    const d=document.createElement("div"); d.className="prof"+(p.id===state.activeProfile?" active":"");
    d.innerHTML=`<strong>${p.name}</strong><span class="muted">fx ${p.effect_id}</span>`;
    const b=document.createElement("button"); b.className="btn"; b.textContent="Activate";
    b.onclick=async()=>{ await invoke("set_active_profile",{id:p.id}); state.activeProfile=p.id; $("#prof-json").value=JSON.stringify(p,null,2); loadProfiles(); refreshStatus(); };
    d.appendChild(b); pl.appendChild(d);
  });
  if(state.profiles[0]) $("#prof-json").value=JSON.stringify(state.profiles[state.activeProfile]||state.profiles[0],null,2);
}
$("#btn-prof-refresh").onclick=loadProfiles;
$("#btn-save-fw").onclick=async()=>{ await invoke("save_profile_fw"); refreshStatus(); };
$("#btn-export").onclick=()=>{ const blob=new Blob([$("#prof-json").value],{type:"application/json"}); const a=document.createElement("a"); a.href=URL.createObjectURL(blob); a.download="mk730-profile.json"; a.click(); };
$("#file-import").onchange=e=>{ const f=e.target.files[0]; if(!f)return; const r=new FileReader(); r.onload=()=>$("#prof-json").value=r.result; r.readAsText(f); };
$("#btn-apply-profile").onclick=async()=>{ try{ const p=JSON.parse($("#prof-json").value); await invoke("apply_profile_to_device",{p}); await invoke("upsert_profile",{p}); loadProfiles(); refreshStatus(); }catch(err){ alert("bad JSON: "+err); } };

// macros
async function loadMacros(){
  state.macros = await invoke("get_macros");
  const ml=$("#mlist"); ml.innerHTML="";
  if(!state.macros.length) ml.innerHTML='<p class="muted">No macros yet. Create one with a trigger key.</p>';
  state.macros.forEach(m=>{
    const d=document.createElement("div"); d.className="mac";
    d.innerHTML=`<strong>${m.trigger}</strong><span class="muted">${m.name} · ${m.events.length} ev · ${m.fw_state}</span>`;
    d.onclick=()=>{ state.selMacro=m.id; $("#m-name").value=m.name; $("#m-trigger").value=m.trigger; $("#m-repeat").value=m.repeat||0; $("#m-fw").textContent=m.fw_state; $("#m-events").value=(m.events||[]).map(e=>`0x${e.hid.toString(16).padStart(2,"0")} ${e.pressed?"down":"up"} ${e.delay_ms}`).join("\n"); };
    ml.appendChild(d);
  });
}
$("#btn-m-new").onclick=()=>{ const t=($("#m-trigger").value||"F5").toUpperCase(); state.selMacro=null; $("#m-name").value="Macro "+t; $("#m-events").value="0x04 down 0\n0x04 up 120"; $("#m-fw").textContent="local_only"; };
$("#btn-m-save").onclick=async()=>{
  const trigger=($("#m-trigger").value||"F5").toUpperCase();
  const lines=$("#m-events").value.split("\n").map(s=>s.trim()).filter(Boolean);
  const events=[];
  for(const ln of lines){
    const m=/0x([0-9a-fA-F]+)\s+(down|up|press)\s*(\d+)?/.exec(ln);
    if(!m){ alert("bad line: "+ln); return; }
    const hid=parseInt(m[1],16), pressed=m[2]!=="up", delay_ms=+(m[3]||0);
    events.push({hid,pressed,delay_ms,modifier:0});
    if(m[2]==="press"){ events.push({hid,pressed:false,delay_ms:80,modifier:0}); }
  }
  const ex = state.macros.find(x=>x.id===state.selMacro);
  const obj={ id: ex?.id || ("m_"+Date.now()), name: $("#m-name").value||("Macro "+trigger), trigger, events, repeat:+$("#m-repeat").value||0, fw_state:"local_only" };
  state.macros = await invoke("save_macro",{m:obj}); state.selMacro=obj.id; loadMacros(); refreshStatus();
};
$("#btn-m-del").onclick=async()=>{ if(!state.selMacro)return; state.macros=await invoke("delete_macro",{id:state.selMacro}); state.selMacro=null; loadMacros(); };

// device
$("#btn-scan").onclick=async()=>{
  const ds=await invoke("list_devices");
  $("#devs").innerHTML=ds.map(d=>`<div class="prof"><strong>${d.product}</strong><span class="muted">VID ${d.vid.toString(16)} PID ${d.pid.toString(16)} IF${d.interface} ${d.demo?"· demo":""}</span></div>`).join("");
  refreshStatus();
};
$("#btn-mode-fw").onclick=async()=>{ await invoke("set_mode",{mode:0}); refreshStatus(); };
$("#btn-mode-fx").onclick=async()=>{ await invoke("set_mode",{mode:1}); refreshStatus(); };
$("#btn-mode-man").onclick=async()=>{ await invoke("set_mode",{mode:2}); refreshStatus(); };
$("#btn-mode-prof").onclick=async()=>{ await invoke("set_mode",{mode:3}); refreshStatus(); };

(async function boot(){
  await loadKeys(); await loadProfiles(); await loadMacros(); $("#btn-scan").click(); refreshStatus();
  setInterval(refreshStatus, 4000);
})();
