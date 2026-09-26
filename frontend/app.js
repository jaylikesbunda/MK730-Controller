const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

// Prefer Tauri's internal bridge, then the configured global API.
let IPC = null;
function detectIpc(){
  if (window.__TAURI_INTERNALS__?.invoke) return "internals";
  if (window.__TAURI__?.invoke) return "global";
  if (window.__TAURI__?.core?.invoke) return "core";
  return null;
}
const invoke = async (cmd, args = {}) => {
  if (!IPC) {
    IPC = detectIpc();
    console.log("[ipc] mode=" + IPC);
    if (!IPC) setStatus("No app bridge - restart the app", true);
  }
  try {
    if (IPC === "internals") return await window.__TAURI_INTERNALS__.invoke(cmd, args);
    if (IPC === "global") return await window.__TAURI__.invoke(cmd, args);
    if (IPC === "core") return await window.__TAURI__.core.invoke(cmd, args);
  } catch (e) {
    // Surface IPC failures instead of pretending the device is absent.
    console.error("[ipc] " + cmd + " failed:", e);
    setStatus("App link error: " + (e && e.message ? e.message : e), true);
    throw e;
  }
  return mock(cmd, args);
};

function setStatus(text, isError){
  const el = $("#status");
  if (el) { el.textContent = text; el.style.color = isError ? "#fca5a5" : ""; }
}

// ---- local demo fallback (so UI works in browser preview) ----
let demoKeys = [];
let demoColors = {};
async function mock(cmd, args) {
  if (cmd === "get_keymap") {
    if (!demoKeys.length) {
      const rows = [
        ["Esc","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12","Prt","Scr","Pse"],
        ["~","1","2","3","4","5","6","7","8","9","0","-","=","⌫","Ins","Hom","PgU"],
        ["Tab","Q","W","E","R","T","Y","U","I","O","P","[","]","\\","Del","End","PgD"],
        ["Caps","A","S","D","F","G","H","J","K","L",";","'","Enter"],
        ["Shift","Z","X","C","V","B","N","M",",",".","/","Shift","▲"],
        ["Ctrl","Win","Alt","Space","Alt","Fn","Menu","Ctrl","◀","▼","▶"]
      ];
      let id = 0;
      demoKeys = rows.flatMap((labels, row) => labels.map((label, col) => ({
        id: id++, label, row, col,
        w: label === "Space" ? 6.25 : label === "Enter" ? 2.25 : label === "⌫" ? 2 : label === "Tab" ? 1.5 : label === "Caps" ? 1.75 : label === "Shift" ? (col === 0 ? 2.25 : 2.75) : 1,
        cls: (row <= 2 && col >= labels.length - 3) || (row >= 4 && col >= labels.length - 1) ? "mini" : ""
      })));
    }
    return demoKeys;
  }
  if (cmd === "get_status") return { connected: false, demo: true, firmware: "browser-demo", mode: 1, active_profile: 0, log_tail: ["browser preview: backend not attached"] };
  if (cmd === "list_devices") return [{ vid: 0x2516, pid: 0x008f, interface: 1, path: "demo://mk730", product: "MK730 (browser demo)", demo: true }];
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
    if (st.demo) {
      pill.textContent = "Not connected";
      pill.className = "pill demo";
      setStatus("Not connected — plug in your keyboard and choose Check again in Device.");
    } else {
      pill.textContent = "Connected";
      pill.className = "pill live";
      setStatus(`Connected · Profile ${st.active_profile+1} · Ready`);
    }
    const tail = (st.log_tail||[]).filter(Boolean);
    const el = $("#log");
    if (el) el.textContent = tail.length ? "Ready. Your changes apply automatically." : "Ready.";
  }catch(e){ setStatus("Something went wrong starting up.", true); }
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
    const div = document.createElement("div"); div.className="krow"; div.dataset.row=r;
    const alignedRow=+r>=3;
    const main=document.createElement("div");main.className="main-keys";
    const arrows=document.createElement("div");arrows.className="arrow-keys";
    rows[r].sort((a,b)=>a.col-b.col).forEach((k,i)=>{
      // The firmware map has LED positions, not physical key spacing. The
      // spacers reproduce the actual MK730 TKL function/nav/arrow clusters.
      const gap = (r==0 && [1,5,9,13].includes(i)) ||
                  ((r==1 || r==2) && i==rows[r].length-3);
      if(gap){ const s=document.createElement("span"); s.className="key-gap"; div.appendChild(s); }
      const b = document.createElement("button");
      b.className = "key "+(k.cls||""); b.textContent = k.label;
      b.style.flexGrow = k.w; b.dataset.led = k.id;
      b.style.background = keyColor(k.id);
      b.title="Click to preview a key response in reactive effects";
      b.onclick = ()=>{if($("#editor-effects").classList.contains("active")){if(REACTIVE_PREVIEW.has(state.fx))simulatePreviewKey(k.id);}else onKey(b,k.id);};
      if(r==4 && k.label==="▲") arrows.appendChild(b);
      else if(r==5 && i>=8) arrows.appendChild(b);
      else if(alignedRow) main.appendChild(b);
      else div.appendChild(b);
    });
    if(alignedRow){div.appendChild(main);if(+r>=4)div.appendChild(arrows);}
    kbd.appendChild(div);
  });
  requestAnimationFrame(measurePreviewCoordinates);
}
const previewPositions={};
function measurePreviewCoordinates(){
  const bounds=$("#kbd").getBoundingClientRect();
  if(!bounds.width||!bounds.height)return;
  $$("#kbd .key").forEach(el=>{const r=el.getBoundingClientRect();previewPositions[+el.dataset.led]={x:(r.left+r.width/2-bounds.left)/bounds.width,y:(r.top+r.height/2-bounds.top)/bounds.height};});
}
window.addEventListener("resize",measurePreviewCoordinates);

const themeButton = $("#btn-theme");
function setTheme(theme){
  document.documentElement.dataset.theme=theme;
  localStorage.setItem("mk730-theme",theme);
  themeButton.title=theme==="dark"?"Switch to light theme":"Switch to dark theme";
  themeButton.setAttribute("aria-label",themeButton.title);
}
setTheme(localStorage.getItem("mk730-theme") || (matchMedia("(prefers-color-scheme: dark)").matches?"dark":"light"));
themeButton.onclick=()=>setTheme(document.documentElement.dataset.theme==="dark"?"light":"dark");

function onKey(btn, led){
  let [r,g,b]=parseColor($("#pick").value);
  if(state.tool==="erase"){ r=g=b=0; }
  state.colors[led]=`rgb(${r},${g},${b})`;
  btn.style.background = keyColor(led);
  schedulePush();
}

let pushTimer = null;
let paintPending = false;
let paintPump = null;
const layered = { active:false, task:null, previewMap:null, generation:0 };
function schedulePush(){
  if(layered.active) return; // The next composed frame includes this edit.
  paintPending = true;
  clearTimeout(pushTimer);
  pushTimer = setTimeout(()=>{ pushFullMap().catch(()=>{}); }, 100);
}
function buildColorMap(){
  const arr = Array.from({length:255},()=>[0,0,0]);
  const brightness = +($("#bright").value||100) / 100;
  for(let id=0;id<255;id++){
    const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(state.colors[id]||"");
    if(m) arr[id]=[1,2,3].map(i=>Math.round(+m[i]*brightness));
  }
  return arr;
}
async function pushFullMap(){
  if(layered.active) return; // The continuous compositor is the single writer.
  paintPending = true;
  clearTimeout(pushTimer);
  if(paintPump) return paintPump;
  paintPump = (async()=>{
    while(paintPending){
      paintPending = false;
      await invoke("set_colormap",{colors:buildColorMap()});
    }
  })();
  try{
    await paintPump;
    setStatus("Lighting applied.");
  }catch(e){
    paintPending = false;
    setStatus("Lighting failed: " + (e?.message||e), true);
    throw e;
  }finally{
    paintPump = null;
  }
}
async function runLightingAction(button, progress, done, action){
  if(button.disabled) return;
  const label=button.textContent;
  button.disabled=true;
  button.textContent=progress;
  setStatus(progress);
  try{ await action(); setStatus(done); }
  catch(e){ setStatus("Lighting failed: " + (e?.message||e), true); }
  finally{ button.disabled=false; button.textContent=label; }
}

// sidebar nav
$$(".nav").forEach(t=>t.onclick=()=>{
  $$(".nav").forEach(x=>x.classList.remove("active"));
  $$(".panel").forEach(x=>x.classList.remove("active"));
  t.classList.add("active");
  $("#tab-"+t.dataset.tab).classList.add("active");
});
$$('.inspector-tab').forEach(tab=>tab.onclick=()=>{
  $$('.inspector-tab').forEach(x=>{
    const active = x === tab;
    x.classList.toggle('active', active);
    x.setAttribute('aria-selected', String(active));
  });
  $$('.editor-pane').forEach(x=>x.classList.toggle('active', x.id === 'editor-'+tab.dataset.editor));
  syncKeyboardPreview();
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
    x.style.background = layered.active && layered.previewMap
      ? `rgb(${layered.previewMap[led].join(",")})` : keyColor(led);
  });
}
$("#btn-fill-all").onclick = async()=>{
  const button=$("#btn-fill-all");
  await runLightingAction(button,"Filling…","All keys filled.",async()=>{
    // A layered effect owns the HID writer and causes pushFullMap() to return
    // early. Stop it and drain any queued paint before applying a solid fill.
    await stopLayered();
    $("#layer-paint").checked=false;
    clearTimeout(pushTimer);
    paintPending=false;
    if(paintPump) await paintPump;

    const [r,g,b]=parseColor($("#pick").value);
    const color=`rgb(${r},${g},${b})`;
    for(let id=0;id<255;id++) state.colors[id]=color;
    repaintKeys();
    await pushFullMap();
  });
};
$("#btn-clear").onclick = async()=>{
  const button=$("#btn-clear");
  await runLightingAction(button,"Turning off…","Lighting turned off.",async()=>{
    await stopLayered();
    $("#layer-paint").checked=false;
    state.colors={}; repaintKeys();
    await pushFullMap();
  });
};
$("#btn-apply-map").onclick = async()=>{
  await runLightingAction($("#btn-apply-map"),"Applying…","Lighting applied.",pushFullMap);
};
// brush tools
$$("#tool-seg .seg-btn").forEach(b=>b.onclick=()=>{
  $$("#tool-seg .seg-btn").forEach(x=>x.classList.remove("active"));
  b.classList.add("active"); state.tool=b.dataset.tool;
});
const SW=["#476ce0","#16b8c6","#42c789","#e9be42","#ef9152","#e66d70","#bc78d8","#ffffff"];
const picker=$("#paint-picker"), plane=$("#pick-plane");
let pickerStartValue="#42d6c4",pickerTarget="#pick",hue=170,saturation=.69,value=.84,draggingPlane=false;
function updatePaintColor(){
  const color=$("#pick").value.toUpperCase();
  $("#color-hex").textContent=color;
  $("#pick-open").style.background=color;
  if(pickerTarget==="#pick")$("#pick-hex").value=color;
}
function setPickerTargetColor(color){
  $(pickerTarget).value=color.toLowerCase();
  $("#pick-hex").value=color.toUpperCase();
  if(pickerTarget==="#pick")updatePaintColor();
  else $("#"+pickerTarget.slice(1)+"-open").style.background=color;
}
function hsvHex(h,s,v){
  const f=n=>{const k=(n+h/60)%6;return Math.round((v-v*s*Math.max(Math.min(k,4-k,1),0))*255).toString(16).padStart(2,"0");};
  return `#${f(5)}${f(3)}${f(1)}`;
}
function hexHsv(color){
  const [r,g,b]=parseColor(color).map(x=>x/255), max=Math.max(r,g,b), min=Math.min(r,g,b), d=max-min;
  let h=0;
  if(d){if(max===r)h=60*(((g-b)/d)%6);else if(max===g)h=60*((b-r)/d+2);else h=60*((r-g)/d+4);}
  return [(h+360)%360,max===0?0:d/max,max];
}
function renderPaintPicker(){
  plane.style.background=`linear-gradient(to top,#000,transparent),linear-gradient(to right,#fff,hsl(${hue} 100% 50%))`;
  $("#pick-marker").style.left=`${saturation*100}%`;
  $("#pick-marker").style.top=`${(1-value)*100}%`;
  $("#pick-hue").value=hue;
  $("#pick-hue").style.setProperty("--hue",`hsl(${hue} 100% 50%)`);
  setPickerTargetColor(hsvHex(hue,saturation,value));
}
function setPlaneFromPointer(e){
  const rect=plane.getBoundingClientRect();
  saturation=Math.max(0,Math.min(1,(e.clientX-rect.left)/rect.width));
  value=1-Math.max(0,Math.min(1,(e.clientY-rect.top)/rect.height));
  renderPaintPicker();
}
function openPaintPicker(target="pick"){
  pickerTarget="#"+target;
  pickerStartValue=$(pickerTarget).value;
  [hue,saturation,value]=hexHsv(pickerStartValue);
  picker.hidden=false;
  const anchor=$("#"+target+"-open"), rect=anchor.getBoundingClientRect(), pad=10;
  const left=Math.max(pad,Math.min(rect.left,innerWidth-picker.offsetWidth-pad));
  let top=rect.bottom+8;
  if(top+picker.offsetHeight>innerHeight-pad)top=Math.max(pad,rect.top-picker.offsetHeight-8);
  picker.style.left=`${left}px`; picker.style.top=`${top}px`;
  renderPaintPicker();
}
function closePaintPicker(commit){
  if(!commit)setPickerTargetColor(pickerStartValue);
  picker.hidden=true;
  draggingPlane=false;
  $("#"+pickerTarget.slice(1)+"-open").focus();
}
$("#pick-open").onclick=()=>openPaintPicker();
$("#fx-c1-open").onclick=()=>openPaintPicker("fx-c1");
$("#fx-c2-open").onclick=()=>openPaintPicker("fx-c2");
$("#fx-c1-open").style.background=$("#fx-c1").value;
$("#fx-c2-open").style.background=$("#fx-c2").value;
$("#pick-close").onclick=$("#pick-cancel").onclick=()=>closePaintPicker(false);
$("#pick-done").onclick=()=>closePaintPicker(true);
$("#pick-hue").oninput=e=>{hue=+e.target.value;renderPaintPicker();};
$("#pick-hex").onchange=e=>{
  const color=e.target.value.trim();
  if(/^#[0-9a-fA-F]{6}$/.test(color))[hue,saturation,value]=hexHsv(color),renderPaintPicker();
  else e.target.value=$(pickerTarget).value.toUpperCase();
};
plane.onpointerdown=e=>{draggingPlane=true;plane.setPointerCapture(e.pointerId);setPlaneFromPointer(e);};
plane.onpointermove=e=>{if(draggingPlane)setPlaneFromPointer(e);};
plane.onpointerup=()=>{draggingPlane=false;};
document.addEventListener("pointerdown",e=>{if(!picker.hidden&&!picker.contains(e.target)&&!e.target.closest(".color-chip,.effect-color-chip,.custom-swatch"))closePaintPicker(false);});
document.addEventListener("keydown",e=>{if(e.key==="Escape"&&!picker.hidden){e.preventDefault();closePaintPicker(false);}});
const sw=$("#swatches"); SW.forEach(c=>{ const d=document.createElement("button"); d.type="button";d.className="sw";d.style.background=c;d.title=c;d.setAttribute("aria-label",`Use ${c}`);d.onclick=()=>{ $("#pick").value=c;updatePaintColor(); }; sw.appendChild(d); });
const customSwatch=document.createElement("button");customSwatch.type="button";customSwatch.className="sw custom-swatch";customSwatch.textContent="+";customSwatch.title="Choose a custom color";customSwatch.setAttribute("aria-label","Choose a custom color");customSwatch.onclick=()=>openPaintPicker();sw.appendChild(customSwatch);
updatePaintColor();
$("#bright").oninput = e=>{ $("#bright-v").textContent=e.target.value+"%"; repaintKeys(); schedulePush(); };
// lightbar groups (firmware LED values)
const BARS={Left:[1,2,3,4],Right:[141,142,143,144],Front:[13,20,27,34,41,55,62,69,76,90,104,111,118,125],Logo:[69]};
$$("[data-bar]").forEach(b=>b.onclick=()=>{
  const [r,g,bl]=parseColor($("#pick").value);
  (BARS[b.dataset.bar]||[]).forEach(led=>{ state.colors[led]=`rgb(${r},${g},${bl})`; });
  schedulePush();
});

// effects (V2 firmware values)
const FX=[[1,"Custom paint"],[4,"Steady"],[5,"Breathing"],[6,"Color cycle"],[7,"Wave"],[8,"Ripple"],[9,"Crosshair"],[10,"Rain"],[11,"Stars"],[21,"Snow"],[20,"Fireball"],[19,"Heartbeat"],[23,"Water ripple"],[16,"Reactive fade"],[17,"Reactive punch"],[22,"Circle spectrum"],[18,"Reactive tornado"],[13,"Customized"],[24,"Off"]];
const LAYERED_FX = new Set([4,5,6,7,22]);
const NEEDS_DIR = new Set([7,18,22]);
const NEEDS_COLOR = new Set([4,5,8,9,10,11,21,20,19,23,16,17]);
function renderFxGrid(){
  const g=$("#fx-grid"); if(!g) return; g.innerHTML="";
  FX.forEach(([v,l])=>{
    const d=document.createElement("div");
    d.className="fx"+(state.fx===v?" active":"");
    d.innerHTML=`<b>${l}</b>`;
    d.onclick=()=>{ state.fx=v; renderFxGrid(); syncFxOptions(); syncKeyboardPreview(); };
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
  $("#fx-bright-row").style.display=(state.fx===1||state.fx===24)?"none":"";
  if((state.fx===18||state.fx===22)&&(state.fxDir===2||state.fxDir===6))state.fxDir=0;
  $$("#fx-dir-seg .seg-btn").forEach(b=>{
    b.hidden=(state.fx===18||state.fx===22)&&(b.dataset.v==="2"||b.dataset.v==="6");
    b.classList.toggle("active",+b.dataset.v===state.fxDir);
  });
  const layer=$("#layer-paint");
  layer.disabled=!LAYERED_FX.has(state.fx);
  layer.title=layer.disabled?"Available with Steady, Breathing, Color cycle, Wave, and Circle spectrum":"";
  $("#layer-row").classList.toggle("unavailable",layer.disabled);
}
function hsv(h,s,v){
  const f=n=>{const k=(n+h*6)%6;return Math.round((v-v*s*Math.max(Math.min(k,4-k,1),0))*255);};
  return [f(5),f(3),f(1)];
}
function mixColor(a,b,t){return a.map((v,i)=>Math.round(v*(1-t)+b[i]*t));}
let previewHit=null;
const REACTIVE_PREVIEW=new Set([8,9,16,17,18,23]);
function simulatePreviewKey(id){previewHit={id,started:performance.now()};}
function composedFrame(now, overlayPaint=false){
  const c1=parseColor($("#fx-c1").value), c2=parseColor($("#fx-c2").value);
  const strength=+$("#fx-bright").value/255, speed=0.35+state.fxLevel*0.28, t=now/1000*speed;
  const directMap=state.fx===1?buildColorMap():null;
  const ids=state.keys.map(k=>k.id), xs=ids.map(id=>previewPositions[id]?.x??.5), ys=ids.map(id=>previewPositions[id]?.y??.5);
  const cx=(Math.min(...xs)+Math.max(...xs))/2, cy=(Math.min(...ys)+Math.max(...ys))/2;
  const maxR=Math.max(...ids.map(id=>Math.hypot((previewPositions[id]?.x??cx)-cx,(previewPositions[id]?.y??cy)-cy)),.1);
  const out=Array.from({length:255},()=>[0,0,0]);
  let hit=null,age=0;
  if(previewHit){age=(now-previewHit.started)/1000;if(age<2)hit=previewPositions[previewHit.id];else previewHit=null;}
  const smooth=(a,b,x)=>{const q=Math.max(0,Math.min(1,(x-a)/(b-a)));return q*q*(3-2*q);};
  for(const k of state.keys){
    const pos=previewPositions[k.id]||{x:.5,y:.5},x=pos.x,y=pos.y;
    const dx=x-cx,dy=y-cy,r=Math.hypot(dx,dy)/maxR,angle=Math.atan2(dy,dx)/(Math.PI*2);
    let rgb=[0,0,0];
    switch(state.fx){
      case 1: rgb=directMap[k.id]||rgb; break;
      case 4: rgb=c1; break;
      case 5: rgb=c1.map(v=>Math.round(v*(Math.sin(t*Math.PI*2)+1)/2)); break;
      case 6: rgb=hsv((t*.075+1)%1,1,1); break;
      case 7:{const axis=state.fxDir===4?1-x:state.fxDir===2?y:state.fxDir===6?1-y:x;const phase=((axis*1.35-t*.34)%1+1)%1;const band=Math.max(0,1-Math.abs(phase-.5)/.16);rgb=[255,255,255].map(v=>Math.round(v*band));break;}
      case 22: rgb=hsv((angle+(state.fxDir===4?-1:1)*t*.08+1)%1,1,1); break;
      case 24: rgb=[0,0,0]; break;
      case 13: rgb=[0,0,0]; break;
      case 19:{const pulse=Math.pow(Math.max(0,Math.sin(t*2.3)),10);rgb=mixColor(c2,c1,pulse);break;}
      case 10: case 11: case 21: case 20:{
        const beat=Math.floor(t*(state.fx===20?2.4:state.fx===11?1.8:1.1));
        const seed=Math.abs(Math.sin((k.id+4)*127.1+beat*311.7)*43758.5453)%1;
        let glow=0;
        if(state.fx===10)glow=seed>.91?Math.max(0,1-((t*1.15+y*1.9)%1))*(.5+seed*.5):0;
        else if(state.fx===21)glow=seed>.86?Math.max(0,1-((t*.42+y*1.2)%1))*(.35+seed*.65):0;
        else if(state.fx===11)glow=seed>.86?Math.max(0,Math.sin(t*4+seed*20)):0;
        else {const flame=Math.max(0,1-Math.abs(x-(cx+Math.sin(t*.42+y*2)*.22))/.27);glow=flame*Math.max(0,1-y*.72)*(.45+.55*Math.max(0,Math.sin(t*2+y*8)));}
        rgb=mixColor(c2,c1,glow);break;
      }
      default:
        if(REACTIVE_PREVIEW.has(state.fx)&&hit){
          const d=Math.hypot(x-hit.x,y-hit.y)/maxR,travel=age*(.65+state.fxLevel*.18);
          let glow=0;
          if(state.fx===8)glow=(1-smooth(.045,.15,Math.abs(d-travel)))*Math.max(0,1-age/1.8);
          else if(state.fx===23)glow=(1-smooth(.03,.2,Math.abs(d-travel*.72)))*Math.max(0,1-age/2.2);
          else if(state.fx===9)glow=Math.max(1-smooth(.02,.12,Math.abs(x-hit.x)),1-smooth(.02,.12,Math.abs(y-hit.y)))*Math.max(0,1-age/1.3);
          else if(state.fx===16)glow=Math.exp(-d*3.5)*Math.max(0,1-age/1.6);
          else if(state.fx===17)glow=Math.exp(-age*7)*Math.max(0,1-d*1.2);
          else {const spiral=Math.atan2(y-hit.y,x-hit.x)/(Math.PI*2)*(state.fxDir===4?-1:1);glow=Math.exp(-Math.abs(d-(.12+age*.2+spiral*.22))*12)*Math.max(0,1-age/1.8);}
          rgb=state.fx===18?mixColor([0,0,0],[90,185,255],glow):mixColor(c2,c1,glow);
        }else rgb=c2;
    }
    out[k.id]=rgb.map(v=>Math.round(v*strength));
  }
  const brightness=+$("#bright").value/100;
  const opacity=+$("#layer-opacity").value/100;
  if(overlayPaint) for(const [id,value] of Object.entries(state.colors)){
    const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(value);
    if(m && +id<255) out[+id]=[1,2,3].map((i)=>Math.round(out[+id][i-1]*(1-opacity)+(+m[i]*brightness)*opacity));
  }
  return out;
}
let previewRaf=0;
function paintPreviewFrame(ts){
  previewRaf=0;
  if(!$("#editor-effects").classList.contains("active")||layered.active)return;
  const frame=state.fx===1?buildColorMap():composedFrame(ts,$("#layer-paint").checked && LAYERED_FX.has(state.fx));
  $$("#kbd .key").forEach(el=>{const rgb=frame[+el.dataset.led]||[0,0,0];el.style.background=`rgb(${rgb.join(",")})`;});
  previewRaf=requestAnimationFrame(paintPreviewFrame);
}
function syncKeyboardPreview(){
  const hint=$("#preview-hint");
  const effectsActive=$("#editor-effects").classList.contains("active");
  if(hint){hint.hidden=!effectsActive||(!REACTIVE_PREVIEW.has(state.fx)&&state.fx!==13);hint.textContent=state.fx===13?"Uses the keyboard's saved effect; preview unavailable":"Click a key to preview its reactive response";}
  if(previewRaf){cancelAnimationFrame(previewRaf);previewRaf=0;}
  if($("#editor-effects").classList.contains("active")&&!layered.active)previewRaf=requestAnimationFrame(paintPreviewFrame);
  else repaintKeys();
}
$("#layer-opacity").oninput=e=>$("#layer-opacity-v").textContent=e.target.value+"%";
$("#layer-paint").onchange=syncKeyboardPreview;
async function stopLayered(){
  layered.active=false;
  layered.generation++;
  if(layered.task) await layered.task;
  layered.task=null;
  layered.previewMap=null;
  syncKeyboardPreview();
}
async function startLayered(){
  await stopLayered();
  clearTimeout(pushTimer); paintPending=false;
  if(paintPump) await paintPump;
  layered.active=true;
  const generation=++layered.generation;
  let firstResolve,firstReject;
  const first=new Promise((resolve,reject)=>{firstResolve=resolve;firstReject=reject;});
  layered.task=(async()=>{
    let sent=false;
    try{
      while(layered.active && generation===layered.generation){
        const frame=composedFrame(performance.now(),true);
        await invoke("set_colormap",{colors:frame});
        layered.previewMap=frame;
        repaintKeys();
        if(!sent){sent=true;firstResolve();}
        await new Promise(resolve=>setTimeout(resolve,90));
      }
    }catch(e){
      layered.active=false;
      $("#layer-paint").checked=false;
      if(!sent) firstReject(e);
      setStatus("Layered lighting stopped: "+(e?.message||e),true);
    }
  })();
  await first;
  syncKeyboardPreview();
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
$("#fx-bright").oninput=e=>$("#fx-bright-v").textContent=Math.round(+e.target.value/255*100)+"%";
$("#btn-fx-apply").onclick=async()=>{
  const eid=state.fx;
  const level=state.fxLevel||3, dir=state.fxDir||0;
  const brightness=+($("#fx-bright").value||255);
  const c1=parseColor($("#fx-c1").value), c2=parseColor($("#fx-c2").value);
  await runLightingAction($("#btn-fx-apply"),"Applying…","Effect applied.",async()=>{
    await stopLayered();
    clearTimeout(pushTimer);
    paintPending=false;
    if(paintPump) await paintPump;
    if($("#layer-paint").checked && LAYERED_FX.has(eid)) await startLayered();
    else if(eid===1) await pushFullMap();
    else await invoke("set_effect_params",{p:{eid,level,brightness,dir,c1,c2}});
  });
};
renderFxGrid(); syncFxOptions();

// profiles
function fxName(id){ const f=FX.find(x=>x[0]===+id); return f?f[1]:"Custom"; }
async function loadProfiles(){
  const remote = await invoke("get_profiles");
  let saved=[]; try{saved=JSON.parse(localStorage.getItem("mk730-profiles")||"[]");}catch{}
  state.profiles=remote;
  for(const p of saved){if(Number.isInteger(p.id)&&p.id>=0&&p.id<5){state.profiles[p.id]=p;await invoke("upsert_profile",{p});}}
  const persist=()=>localStorage.setItem("mk730-profiles",JSON.stringify(state.profiles));
  const pl=$("#plist"); pl.innerHTML="";
  state.profiles.forEach(p=>{
    const c=p.params?p.params.color1:{r:124,g:58,b:237};
    const d=document.createElement("div"); d.className="prof"+(p.id===state.activeProfile?" active":"");
    d.innerHTML=`<span class="dot" style="background:rgb(${c.r},${c.g},${c.b})"></span><strong>${p.name}</strong><span class="muted">${fxName(p.effect_id)}</span>`;
    const b=document.createElement("button"); b.className="btn"; b.textContent="Load"; b.title="Apply this profile to the keyboard";
    b.onclick=async()=>{try{await stopLayered();await invoke("apply_profile_to_device",{p});state.activeProfile=p.id;state.fx=p.effect_id;state.fxLevel=Math.max(1,Math.min(5,p.params?.p1_speed||3));state.fxDir=p.params?.p2||0;if(p.effect_id===1)state.colors=Object.fromEntries((p.colormap||[]).map((c,i)=>[i,`rgb(${c.join(",")})`]));$("#fx-c1").value=`#${[p.params.color1.r,p.params.color1.g,p.params.color1.b].map(v=>v.toString(16).padStart(2,"0")).join("")}`;$("#fx-c2").value=`#${[p.params.color2.r,p.params.color2.g,p.params.color2.b].map(v=>v.toString(16).padStart(2,"0")).join("")}`;$("#fx-bright").value=Math.round((p.brightness??100)*255/100);$("#bright").value=p.brightness??100;$("#prof-json").value=JSON.stringify(p,null,2);renderFxGrid();syncFxOptions();loadProfiles();syncKeyboardPreview();refreshStatus();setStatus(`${p.name} loaded to keyboard.`);}catch(e){setStatus("Profile load failed: "+(e?.message||e),true);}};
    const s=document.createElement("button");s.className="text-button";s.textContent="Save current";s.title="Save current lighting into this profile";
    s.onclick=async()=>{const direct=state.fx===1;const colors=Array.from({length:255},(_,i)=>{const m=/rgb\((\d+),(\d+),(\d+)\)/.exec(state.colors[i]||"");return m?[+m[1],+m[2],+m[3]]:[0,0,0];});const rgb=color=>{const h=color.replace("#","");return{r:parseInt(h.slice(0,2),16),g:parseInt(h.slice(2,4),16),b:parseInt(h.slice(4,6),16)};};const updated={...p,effect_id:direct?1:state.fx,params:{p1_speed:state.fxLevel,p2:state.fxDir,p3:32,color1:rgb($("#fx-c1").value),color2:rgb($("#fx-c2").value),multilayer:0},colormap:colors,brightness:+$("#bright").value};try{await invoke("upsert_profile",{p:updated});state.profiles[updated.id]=updated;persist();$("#prof-json").value=JSON.stringify(updated,null,2);loadProfiles();setStatus(`${updated.name} saved. Load it to apply it to the keyboard.`);}catch(e){setStatus("Profile save failed: "+(e?.message||e),true);}};
    d.append(b,s); pl.appendChild(d);
  });
  persist();
  if(state.profiles[0]&&!$("#prof-json").value) $("#prof-json").value=JSON.stringify(state.profiles[state.activeProfile]||state.profiles[0],null,2);
}
$("#btn-prof-refresh").onclick=loadProfiles;
$("#btn-save-fw").onclick=async()=>{ try{await invoke("save_profile_fw");setStatus("Current keyboard lighting saved to its firmware slot.");}catch(e){setStatus("Keyboard save failed: "+(e?.message||e),true);} refreshStatus(); };
$("#btn-export").onclick=()=>{ const blob=new Blob([$("#prof-json").value],{type:"application/json"}); const a=document.createElement("a"); a.href=URL.createObjectURL(blob); a.download="mk730-profile.json"; a.click(); };
$("#file-import").onchange=e=>{ const f=e.target.files[0]; if(!f)return; const r=new FileReader(); r.onload=()=>$("#prof-json").value=r.result; r.readAsText(f); };
$("#btn-apply-profile").onclick=async()=>{ try{ const p=JSON.parse($("#prof-json").value); await stopLayered(); await invoke("upsert_profile",{p}); state.profiles[p.id]=p;localStorage.setItem("mk730-profiles",JSON.stringify(state.profiles));await invoke("apply_profile_to_device",{p});state.activeProfile=p.id;loadProfiles();refreshStatus();setStatus(`${p.name} imported and loaded.`); }catch(err){ alert("Profile could not be imported: "+(err?.message||err)); } };

// macros
async function loadMacros(){
  state.macros = await invoke("get_macros");
  let local=[];try{local=JSON.parse(localStorage.getItem("mk730-macros")||"[]");}catch{}
  for(const m of local){if(m&&m.id&&m.trigger&&Array.isArray(m.events)){try{state.macros=await invoke("save_macro",{m});}catch(e){console.warn("Could not restore macro",m.trigger,e);}}}
  const ml=$("#mlist"); if(!ml) return; ml.innerHTML="";
  if(!state.macros.length) ml.innerHTML='<p class="muted">No macros yet. Create one to get started.</p>';
  state.macros.forEach(m=>{
    const d=document.createElement("div"); d.className="mac";
    d.innerHTML=`<strong>${m.trigger}</strong><span class="muted">${m.name} · ${m.events.length} steps · PC hotkey</span>`;
    d.onclick=()=>{ state.selMacro=m.id; $("#m-name").value=m.name; $("#m-trigger").value=m.trigger; $("#m-text").value=""; $("#m-repeat").value=m.repeat||0; $("#m-events").value=(m.events||[]).map(e=>`0x${e.hid.toString(16).padStart(2,"0")} ${e.pressed?"down":"up"} ${e.delay_ms} mod=0x${(e.modifier||0).toString(16).padStart(2,"0")}`).join("\n"); };
    const play=document.createElement("button");play.className="text-button";play.textContent="Run";play.onclick=async e=>{e.stopPropagation();try{await invoke("run_macro",{id:m.id});setStatus(`${m.name} played.`);}catch(err){setStatus("Macro failed: "+(err?.message||err),true);}};d.appendChild(play);
    ml.appendChild(d);
  });
}
$("#btn-m-new").onclick=()=>{ const t=($("#m-trigger").value||"F5").toUpperCase(); state.selMacro=null; $("#m-name").value="Macro "+t; $("#m-text").value=""; $("#m-events").value="0x04 down 0\n0x04 up 120"; };
function textToEvents(text){
  const ev=[]; let t=0;
  for(const ch of text){
    let hid=0,mod=0;
    const code=ch.charCodeAt(0);
    if(ch>="a"&&ch<="z") hid=code-93;
    else if(ch>="A"&&ch<="Z"){hid=code-61;mod=2;}
    else if(ch>="1"&&ch<="9")hid=code-19;
    else if(ch==="0")hid=39;
    else if(ch===" ")hid=44;
    else if(ch==="\n")hid=40;
    else if(ch==="\t")hid=43;
    else {
      const shifted={"!":30,"@":31,"#":32,"$":33,"%":34,"^":35,"&":36,"*":37,"(":38,")":39,"_":45,"+":46,"{":47,"}":48,"|":49,":":51,"\"":52,"~":53,"<":54,">":55,"?":56};
      const plain={"-":45,"=":46,"[":47,"]":48,"\\":49,";":51,"'":52,"`":53,",":54,".":55,"/":56};
      if(shifted[ch]){hid=shifted[ch];mod=2;}
      else if(plain[ch])hid=plain[ch];
      else continue;
    }
    ev.push({hid,pressed:true,delay_ms:t===0?0:60,modifier:mod});
    ev.push({hid,pressed:false,delay_ms:60,modifier:mod});
    t++;
  }
  return ev;
}
const _bmt=$("#btn-m-type"); if(_bmt) _bmt.onclick=()=>{
  const ev=textToEvents($("#m-text").value||"");
  $("#m-events").value=ev.map(e=>`0x${e.hid.toString(16).padStart(2,"0")} ${e.pressed?"down":"up"} ${e.delay_ms} mod=0x${e.modifier.toString(16).padStart(2,"0")}`).join("\n");
};
$("#btn-m-send").onclick=async()=>{
  const text=$("#m-text").value||"";
  if(!text)return setStatus("Enter text to type first.",true);
  const b=$("#btn-m-send"),label=b.textContent;b.disabled=true;b.textContent="Typing…";
  try{await invoke("type_text_now",{text});setStatus("Text sent to the previously active window.");}
  catch(e){setStatus("Could not type text: "+(e?.message||e),true);}
  finally{b.disabled=false;b.textContent=label;}
};
$("#btn-m-save").onclick=async()=>{
  const trigger=($("#m-trigger").value||"F5").toUpperCase();
  const text=$("#m-text").value||"";
  if(text.length>1000){setStatus("Text macros are limited to 1000 characters.",true);return;}
  const lines=$("#m-events").value.split("\n").map(s=>s.trim()).filter(Boolean);
  const events=text?textToEvents(text):[];
  if(!text){
  for(const ln of lines){
    const m=/0x([0-9a-fA-F]+)\s+(down|up|press)\s*(\d+)?(?:\s+mod=0x([0-9a-fA-F]+))?/.exec(ln) || /^([a-z0-9])\s+(down|up)\s*(\d+)?/i.exec(ln);
    if(!m){ alert("Could not understand this line: "+ln); return; }
    let hid, pressed, delay_ms, modifier;
    if(ln.startsWith("0x")||ln.startsWith("0X")){ hid=parseInt(m[1],16); pressed=m[2].toLowerCase()!=="up"; delay_ms=+(m[3]||0); modifier=parseInt(m[4]||"0",16); }
    else { hid=m[1].toLowerCase().charCodeAt(0)-87; pressed=m[2].toLowerCase()!=="up"; delay_ms=+(m[3]||0); modifier=0; }
    events.push({hid,pressed,delay_ms,modifier});
    if(m[2].toLowerCase()==="press"){ events.push({hid,pressed:false,delay_ms:80,modifier}); }
  }
  }
  if(!events.length){setStatus("Add text or at least one key step before saving.",true);return;}
  const ex = state.macros.find(x=>x.id===state.selMacro);
  const obj={ id: ex?.id || ("m_"+Date.now()), name: $("#m-name").value||("Macro "+trigger), trigger, events, repeat:+$("#m-repeat").value||0, fw_state:"global_hotkey" };
  try{state.macros = await invoke("save_macro",{m:obj}); state.selMacro=obj.id; localStorage.setItem("mk730-macros",JSON.stringify(state.macros)); await loadMacros(); setStatus("Saved. Press "+trigger+" anywhere while MK730 Controller is running to type this macro.");}catch(e){setStatus("Could not save macro: "+(e?.message||e),true);}
};
$("#btn-m-del").onclick=async()=>{ if(!state.selMacro)return; state.macros=await invoke("delete_macro",{id:state.selMacro});localStorage.setItem("mk730-macros",JSON.stringify(state.macros)); state.selMacro=null; loadMacros(); };

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
  setStatus("Startup error: " + (e.message||"unknown"), true);
});
window.addEventListener("unhandledrejection", (e)=>{
  const m = e.reason && e.reason.message ? e.reason.message : String(e.reason);
  setStatus("Background error: " + m, true);
});

async function safe(fn, name){
  try{ await fn(); }
  catch(e){ console.warn(name, e); }
}

(async function boot(){
  setStatus("Looking for your keyboard…");
  try {
    await loadKeys();
    await loadProfiles();
    await loadMacros();
    renderDevs(await invoke("list_devices"));
  } catch(e) {
    setStatus("Startup failed: " + (e && e.message ? e.message : e), true);
    return;
  }
  await refreshStatus();
  setInterval(() => { refreshStatus().catch(()=>{}); }, 5000);
})();
