const {Scene, WorkerPool} = wasm_bindgen;
let wasm = null;

let mainCanvas = document.getElementById("screen");
let ctx = mainCanvas.getContext('2d');
let selectorCanvas = document.getElementById("controls");
let selector_ctx = selectorCanvas.getContext('2d');
selector_ctx.lineWidth = 1;
selector_ctx.strokeStyle = '#ff964d';
let historyCanvas = document.getElementById("history");
let history_ctx = historyCanvas.getContext('2d');

const HISTORY_COUNT = 6;
const HISTORY_TOP_MARGIN = 20;
const DEFAULT_DOWNSAMPLE_FACTOR = 5;

/* main renderer*/
let rndr = (callback) => {
    let {scale, dx, dy, iterations, color_mode, color_threads} = controls;
    updateURL();
    let startTime = performance.now();
    let interval = setInterval(() => {
        ctx.putImageData(wasmScene.getBuffer(), 0, 0);
    }, controls.updateEvery);
    wasmScene.render(pool, scale, dx, dy, iterations, color_mode, color_threads).then(_ => {
        clearInterval(interval);
        ctx.putImageData(wasmScene.getBuffer(), 0, 0);
        let duration = performance.now() - startTime;
        document.getElementById("renderTime").innerText = "RenderTime: " + duration.toFixed(2) + "ms";
        if (typeof callback === "function") callback();
    });
    renderHistory();
};

/* rendering state*/
let history = [];
let wasmScene;
let pool;

/** dat.GUI control defaults **/
let Controls = function() {
    this.scale = 305.;
    this.dx = 1;
    this.dy = 0;
    this.iterations = 350;
    this.updateEvery = 10;
    this.num_threads = 8;
    this.color_mode = 2;
    this.animation_num_frames = 100;
    this.color_threads = false;
};
let controls = new Controls();
parseURL();


/*Init GUI*/
let stats;// will init stats on First animation
let gui = new dat.GUI({name:"Fractal Controls", autoPlace: false });
let customContainer = document.getElementById('gui');
customContainer.appendChild(gui.domElement);
// gui.add(controls, 'scale',0,10000).onChange(rndr).listen();
// gui.add(controls, 'dx').onChange(rndr);
// gui.add(controls, 'dy').onChange(rndr);
gui.add(controls, 'iterations',10,5000).onChange(rndr);
gui.add(controls, 'updateEvery',5,100);
gui.add(controls, 'color_mode', { COLOR: 3, COLOR_LIGHT: 2, GRAY_LIGHT: 0, GRAY_DARK: 1}).onChange(rndr);
gui.add(controls, 'num_threads', [1,2,3,4,5,8,13,21]).onChange(() => run(window.innerWidth, window.innerHeight, controls.num_threads));
gui.add(controls, 'animation_num_frames', 10, 300);
gui.add(controls, 'color_threads').onChange(rndr);


async function run(width,height,threads) {
    mainCanvas.width = width;
    mainCanvas.height = height;
    selectorCanvas.width = width;
    selectorCanvas.height = height;
    historyCanvas.width = width/4;
    historyCanvas.height = height;


    wasm = wasm || await wasm_bindgen("./pkg/beh_bg.wasm");
    pool = new WorkerPool(threads);
    wasmScene = new Scene(width, height, threads, pool);

    rndr();
}
run(window.innerWidth, window.innerHeight, controls.num_threads);


/** Handle window resize BEGIN */
let resizeId = null;
window.addEventListener( 'resize', onWindowResize, false );
function onWindowResize() {
    clearInterval(resizeId);
    resizeId = setTimeout(onWindowResizeDone, 500);
}

function onWindowResizeDone(){
    let canvas = document.getElementById("screen");

    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    console.log("Window Resize Called");
    run(window.innerWidth, window.innerHeight, controls.num_threads);

}
/** Handle window resize END */



/** Handle HISTORY BEGIN **/
// simple downscaling procedure, todo:: move to wasm
function downscale(imgData, factor) {
    let {data, width, height} = imgData;
    let res = new Uint8ClampedArray(Math.floor(width/factor) * Math.floor(height/factor) * 4);
    let ind = 0;
    for (let i = 0; i < data.length; i += 4) {
        let y = Math.floor((i/4) / width);
        if (y % factor !== 0) continue;
        // ignore the first row unless hight is a multiple of factor: weird corner case given Math.floor(w/f) above
        if (y === 0 && height % factor !== 0) continue;
        let x = Math.floor(i/4) % width;
        if (x % factor !== 0) continue;
        // ignore the first col unless width is a multiple of factor: weird corner case given Math.floor(h/f) above
        if (x === 0 && width % factor !== 0) continue;

        res[ind] = data[i]+10;
        res[ind+1] = data[i+1]*1.1;
        res[ind+2] = data[i+2]+30;
        res[ind+3] = data[i+3];
        ind += 4;
    }
    console.assert(Math.floor(width/factor) * Math.floor(height/factor) * 4  === ind);
    return res;
}

let HistoryItem = function(controls, width, height, factor) {
    this.controls = Object.assign({}, controls);
    factor = factor || DEFAULT_DOWNSAMPLE_FACTOR;
    // this.width = width/factor; // unused
    this.height = height/factor;
    let arr = downscale(ctx.getImageData(0,0, width, height), factor);
    this.buffer = new ImageData(arr, Math.floor(width/factor), Math.floor(height/factor));
};

let renderHistory = function() {
    history_ctx.clearRect(0,0, historyCanvas.width, historyCanvas.height);
    let lastInd = Math.max(0, history.length - HISTORY_COUNT);
    for (let i = history.length - 1; i >= lastInd; i--) {
        history_ctx.putImageData(history[i].buffer, 0,HISTORY_TOP_MARGIN + (history.length - 1 - i) * history[i].height);
    }
};

historyCanvas.onclick = function(e) {
    if (history.length < 1) return;
    let height = history[0].height;
    let clickY = e.clientY;
    let ind = Math.floor((clickY - HISTORY_TOP_MARGIN)/height)+1;
    let step = () => {
        if(ind-- > 0) {
            undo();
            rndr();
            setTimeout(step, 800);
        }

    };
    step();
};
/** Handle HISTORY END **/


/** Handle SELECTOR BOX BEGIN */
let box = null;
window.onmousedown = function(e)
{
    if ( box == null )
        // create a box with zero area to make sure we do not zoom in when there is just a click
        box = [e.clientX, e.clientY, e.clientX, e.clientY];
};

window.onmousemove = function(e)
{
    if ( box != null ) {
        selector_ctx.clearRect(0, 0, selectorCanvas.width, selectorCanvas.height);

        // draw new box
        box[2] = e.clientX;
        box[3] = e.clientY;
        selector_ctx.strokeRect(box[0], box[1], box[2]-box[0], box[3]-box[1]);
    }
};

window.onmouseup = function(e)
{
    let {width,height} = mainCanvas;
    const BOX_AREA_THRESHHOLD = 500;
    selector_ctx.clearRect(0, 0, selectorCanvas.width, selectorCanvas.height);

    if ( box != null ) {
        let area = (box) => Math.abs(box[0] - box[2]) * Math.abs(box[1] - box[3]);
        if (area(box) < BOX_AREA_THRESHHOLD) {
            box = null;
            return;
        }

        let item = new HistoryItem(controls, width, height);
        history.push(item);
        document.getElementById("undo").disabled = false;

        let x =   Math.min(box[0], box[2]) + Math.abs(box[0] - box[2])/2 - width/2;
        let y =   Math.min(box[1], box[3]) + Math.abs(box[1] - box[3])/2 -  height/2;
        controls.dx += -x/controls.scale;
        controls.dy += -y/controls.scale;
        let xScale = Math.abs(box[0]-box[2])/width;
        let yScale = Math.abs(box[1] - box[3])/height;
        controls.scale /= Math.max(xScale, yScale);
        box = null;
        rndr();
    }
};
/** Handle SELECTOR BOX END */


/** Handle history Undo*/
function undo() {
    if (history.length < 1) return;
    let last = history.pop();
    controls = last.controls;
    rndr();
}


/** Handle URL update */
function updateURL() {
    location.hash = "controls=" + JSON.stringify(controls);
}

function parseURL() {
    if(location.hash.length === 0) return;
    try {
        Object.assign(controls, JSON.parse(decodeURIComponent(location.hash.split("#controls=")[1])));
    } catch (e) {
        location.hash = "";
    }
}

function downloadPng() {
    let data = mainCanvas.toDataURL('image/png');
    document.getElementById("download").href = data;
}
let inAnimate = false;
function startAnimate() {
    if(! stats)
    {
        stats = new Stats();
        stats.showPanel(0);
        document.getElementById("stats").appendChild( stats.dom );
    }
    document.getElementById("stats").hidden = false;
    // empty history to not clutter the view
    while(history.length > 0)history.pop();
    gui.close();
    inAnimate = true;
    let globalStep =  Math.max(100, controls.scale/controls.animation_num_frames);
    let animate = function () {
        if (! inAnimate) return;
        if (controls.scale > 250) {
            stats.begin();
            let step = Math.max(100, controls.scale/controls.animation_num_frames);
            if (controls.scale > 10000) step += Math.min(globalStep, controls.scale - step - 100);
            controls.scale -= step;
            rndr(animate);
            stats.end();
        } else {
            inAnimate = false;
            setTimeout(() => {
                document.getElementById("stats").hidden = true;
                gui.open();
            }, 2000 );
        }
    };
    animate();
}

function stopAnimate() {
    inAnimate = false;
    setTimeout(() => {
        document.getElementById("stats").hidden = true;
        gui.open();

    }, 2000 );
}