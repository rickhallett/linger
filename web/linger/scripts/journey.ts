import * as T from 'three/webgpu';
import { pass, uv, sin, uniform, color } from 'three/tsl';
import { bloom } from 'three/addons/tsl/display/BloomNode.js';
import { Application, ParticleContainer, Particle, Texture, Graphics } from 'pixi.js';

const clamp = (n: number, a = 0, b = 1) => Math.max(a, Math.min(b, n));
const smooth = (a: number, b: number, n: number) => { const x = clamp((n - a) / (b - a)); return x * x * (3 - 2 * x); };
const mix = (a: number, b: number, x: number) => a + (b - a) * x;
const words = ['sed', '-n', "'8,16p'", 'src/main.py'];
const definitions = [
  'Read text and apply an editing expression. Here, sed prints a section of a file.',
  'Turn off automatic printing. Let the expression decide which lines appear.',
  'Select lines 8 through 16, and print them.',
  'Read from this file. The path is the last argument in this command.',
];
// Public, authored examples. Never executed; no private recordings are loaded.
const calls = [
  ['git status --short --branch', '## main', ' M src/main.py'],
  ['git diff --stat', 'src/main.py | 8 ++++++--', '1 file changed'],
  ['uv run pytest -q', '........................', '24 passed in 0.18s'],
  ["sed -n '8,16p' src/main.py", 'def mean(items):', '    if not items:', '        return None', '    return sum(items) / len(items)'],
  ['git diff -- src/main.py', '+    if not items:', '+        return None'],
  ['uv run pytest tests/test_mean.py', 'test_empty_list PASSED', 'test_single_item PASSED'],
  ['python3 -', '>>> mean([])', 'None'],
  ["rg -n 'mean' tests", 'tests/test_mean.py:4:def test_empty_list():', 'tests/test_mean.py:9:def test_single_item():'],
  ["sed -n '1,20p' tests/test_mean.py", 'def test_empty_list():', '    assert mean([]) is None', '', 'def test_single_item():', '    assert mean([3]) == 3'],
  ['git diff --check', ''],
  ['uv run ruff check src tests', 'All checks passed!'],
  ["rg -n 'None' src", 'src/main.py:10:        return None'],
  ['python3 -', '>>> mean([2, 4, 6])', '4.0'],
  ['uv run pytest -x', 'tests/test_mean.py ......', '6 passed'],
  ["find tests -name '*.py'", 'tests/test_mean.py', 'tests/test_input.py'],
  ['git status --short', ' M src/main.py', ' M tests/test_mean.py'],
  ["sed -n '1,12p' pyproject.toml", '[project]', 'name = "small-calculations"', 'requires-python = ">=3.11"'],
  ['rg -n TODO src', 'src/main.py:8: # TODO: handle an empty list'],
  ["sed -n '8,16p' src/main.py", 'def mean(items):', '    return sum(items) / len(items)'],
  ['python3 -', '>>> mean([])', 'ZeroDivisionError: division by zero'],
  ['uv run pytest tests/test_mean.py -x', 'FAILED test_empty_list', 'ZeroDivisionError: division by zero'],
  ["rg -n 'len\\(items\\)' src", 'src/main.py:9:    return sum(items) / len(items)'],
  ['git log --oneline -3', 'b71d23f Add input checks', 'b0e40c8 Introduce mean', '349e206 Start small calculations'],
  ["sed -n '1,18p' README.md", '# Small calculations', '', 'A few functions for working with lists.'],
  ["find src -type f -name '*.py'", 'src/main.py', 'src/input.py'],
  ['python3 --version', 'Python 3.12.4'],
  ["rg -n 'def ' src", 'src/main.py:8:def mean(items):', 'src/input.py:3:def parse_line(line):'],
  ['git show --stat HEAD', 'src/input.py | 12 ++++++++++++'],
  ["sed -n '1,12p' src/input.py", 'def parse_line(line):', '    return [float(n) for n in line.split()]'],
  ['uv run pytest --collect-only -q', 'tests/test_mean.py::test_empty_list', 'tests/test_input.py::test_spaces'],
  ['git branch --show-current', 'main'],
  ["sed -n '8,16p' src/main.py", 'def mean(items):', '    return sum(items) / len(items)'],
];

type Glyph = { x: number; y: number };
type Panel = { mesh: T.Mesh; material: T.MeshBasicMaterial; x: number; y: number; z: number; glyphs: Glyph[] };
type Grain = { particle: Particle; sx: number; sy: number; sz: number; tx: number; ty: number; part: number; seed: number; angle: number };

function panelTexture(lines: string[]) {
  const canvas = document.createElement('canvas'); canvas.width = 768; canvas.height = 384;
  const c = canvas.getContext('2d', { willReadFrequently: true })!;
  c.clearRect(0, 0, 768, 384);
  c.font = '26px monospace'; c.textBaseline = 'top';
  lines.forEach((line, i) => { c.fillStyle = i === 0 ? '#dacf91' : '#a3c9ad'; c.fillText(line, 28, 34 + i * 47); });
  const pixels = c.getImageData(0, 0, 768, 384).data;
  const glyphs: Glyph[] = [];
  for (let y = 0; y < 360; y += 5) for (let x = 0; x < 768; x += 5) {
    if (pixels[(y * 768 + x) * 4 + 3] > 110) glyphs.push({ x: (x / 768 - .5) * 6.6, y: (.5 - y / 384) * 3.3 });
  }
  c.globalCompositeOperation = 'destination-over'; c.fillStyle = '#123024'; c.fillRect(0, 0, 768, 384);
  c.globalCompositeOperation = 'source-over'; c.strokeStyle = '#739d7c'; c.lineWidth = 2; c.strokeRect(1, 1, 766, 382);
  const texture = new T.CanvasTexture(canvas); texture.colorSpace = T.SRGBColorSpace; texture.anisotropy = 4;
  return { texture, glyphs };
}

export async function startJourney() {
  const body = document.body;
  const section = document.querySelector<HTMLElement>('.journey-scroll')!;
  const viewport = document.querySelector<HTMLElement>('.journey-viewport')!;
  const threeHost = document.querySelector<HTMLElement>('#journey-three')!;
  const pixelHost = document.querySelector<HTMLElement>('#journey-pixels')!;
  const timeCopy = document.querySelector<HTMLElement>('.time-copy')!;
  const matterCopy = document.querySelector<HTMLElement>('.matter-copy')!;
  const anatomy = document.querySelector<HTMLElement>('.journey-anatomy')!;
  const project = document.querySelector<HTMLElement>('.journey-project')!;
  const nav = document.querySelector<HTMLElement>('.journey-nav')!;
  const cue = document.querySelector<HTMLElement>('.travel-cue')!;
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
  let progress = 0, target = 0, lastScroll = -1;
  let width = viewport.clientWidth, height = viewport.clientHeight;
  let selected = 1, raf = 0, disposed = false;
  const pointer = { x: -10000, y: -10000 };
  const scrollRange = () => Math.max(1, section.offsetHeight - height);
  const onScroll = () => { target = clamp((scrollY - section.offsetTop) / scrollRange()); };
  addEventListener('scroll', onScroll, { passive: true }); onScroll(); progress = target;
  const buttons = [...document.querySelectorAll<HTMLButtonElement>('[data-syntax]')];
  buttons.forEach((button, i) => button.addEventListener('click', () => {
    selected = i; buttons.forEach((b, j) => b.setAttribute('aria-pressed', String(i === j)));
    document.querySelector('#syntax-name')!.textContent = words[i];
    document.querySelector('#syntax-description')!.textContent = definitions[i];
  }));
  addEventListener('pointermove', e => { pointer.x = e.clientX; pointer.y = e.clientY; }, { passive: true });
  function dom(p: number) {
    const daylight = reduced ? (p > .83 ? 1 : 0) : smooth(.79, .965, p);
    body.style.setProperty('--paper-reveal', String(daylight));
    body.style.setProperty('--project-opacity', String(smooth(.83, .97, p)));
    body.style.setProperty('--phase', String(clamp(p / .46)));
    timeCopy.style.opacity = String(1 - smooth(.29, .405, p));
    timeCopy.inert = p > .405;
    matterCopy.style.opacity = String(smooth(.48, .58, p) * (1 - smooth(.76, .84, p)));
    anatomy.style.opacity = String(smooth(.615, .665, p) * (1 - smooth(.77, .835, p)));
    const syntaxActive = p > .625 && p < .825;
    anatomy.inert = !syntaxActive; matterCopy.inert = p < .48 || p > .84;
    project.inert = p < .84; project.classList.toggle('arrived', p > .84);
    cue.style.opacity = String(1 - smooth(.3, .42, p));
    nav.classList.toggle('light', p > (reduced ? .83 : .945));
    viewport.dataset.phase = p < .43 ? 'tunnel' : p < .63 ? 'morph' : p < .79 ? 'syntax' : 'paper';
    viewport.dataset.progress = p.toFixed(3);
    threeHost.style.opacity = String(1 - smooth(.425, .54, p));
    pixelHost.style.opacity = String(smooth(.414, .47, p) * (1 - smooth(.80, .94, p)));
  }
  dom(progress);
  try {
    const renderer = new T.WebGPURenderer({ antialias: true, alpha: false });
    renderer.setClearColor(0x10271d, 1); renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5)); renderer.setSize(width, height);
    await renderer.init(); threeHost.append(renderer.domElement);
    const scene = new T.Scene(); scene.background = new T.Color(0x10271d); scene.fog = new T.FogExp2(0x10271d, .014);
    const camera = new T.PerspectiveCamera(43, width / height, .1, 420);
    const panels: Panel[] = [];
    const tunnel = new T.Group(); scene.add(tunnel);
    const rims: T.LineBasicMaterial[] = [];
    for (let i = 0; i < 42; i++) {
      const z = -i * 7.4;
      const material = new T.LineBasicMaterial({ color: i % 4 === 0 ? 0xc4cf9e : 0x6b9e7c, transparent: true, opacity: i % 4 === 0 ? .72 : .35 });
      const rim = new T.LineSegments(new T.EdgesGeometry(new T.BoxGeometry(12, 7.5, .05)), material);
      rim.position.set(0, 0, z); tunnel.add(rim); rims.push(material);
      if (i < calls.length) {
        const { texture, glyphs } = panelTexture(calls[i]);
        const mat = new T.MeshBasicMaterial({ map: texture, side: T.DoubleSide, transparent: true, opacity: .86 });
        const mesh = new T.Mesh(new T.PlaneGeometry(6.6, 3.3), mat);
        const x = i === 0 ? 2.1 : Math.sin(i * 2.1) * 2.5;
        const y = Math.cos(i * 1.2) * 1.15 - .25;
        mesh.position.set(x, y, z - .15); tunnel.add(mesh);
        panels.push({ mesh, material: mat, x, y, z: z - .15, glyphs });
      }
    }
    viewport.dataset.panels = String(panels.length);
    const shaderTime = uniform(0);
    const railMaterial = new T.MeshBasicNodeMaterial({ transparent: true, side: T.DoubleSide, depthWrite: false });
    railMaterial.colorNode = color('#89b48c');
    railMaterial.opacityNode = sin(uv().y.mul(1600).sub(shaderTime)).mul(.5).add(.5).pow(12).mul(.25);
    for (let i = 0; i < 24; i++) {
      const rail = new T.Mesh(new T.PlaneGeometry(.018, 340), railMaterial);
      rail.rotation.x = -Math.PI / 2; rail.position.set(-5.8 + i * .5, -3.74, -155); tunnel.add(rail);
    }
    const streakCount = 170;
    const streakPositions = new Float32Array(streakCount * 6);
    const streakSeeds = Array.from({ length: streakCount }, (_, i) => ({ x: Math.cos(i * 2.399) * (6 + (i % 11) * .8), y: Math.sin(i * 2.399) * (4 + (i % 7) * .7), z: -(i / streakCount) * 300 }));
    const streakGeometry = new T.BufferGeometry(); streakGeometry.setAttribute('position', new T.BufferAttribute(streakPositions, 3));
    const streakMaterial = new T.LineBasicMaterial({ color: 0xb0c899, transparent: true, opacity: .2 });
    scene.add(new T.LineSegments(streakGeometry, streakMaterial));
    const pipeline = new T.RenderPipeline(renderer);
    const scenePass = pass(scene, camera); const sceneColor = scenePass.getTextureNode('output');
    pipeline.outputNode = sceneColor.add(bloom(sceneColor, .24, .45, .8));

    const app = new Application();
    await app.init({ width, height, autoStart: false, backgroundAlpha: 0, antialias: true, resolution: Math.min(devicePixelRatio, 1.5), autoDensity: true, preference: 'webgpu' });
    app.stop(); pixelHost.append(app.canvas);
    const grains = new ParticleContainer({ dynamicProperties: { position: true, color: true, scale: true, rotation: false } });
    const connector = new Graphics(); app.stage.addChild(grains, connector);
    let dots: Grain[] = [];
    function syntaxTargets() {
      const c = document.createElement('canvas'); c.width = 1500; c.height = 420;
      const ctx = c.getContext('2d', { willReadFrequently: true })!;
      const mobile = width < 700;
      const boxes: { left: number; right: number; top: number; bottom: number; part: number }[] = [];
      ctx.textBaseline = 'middle'; ctx.fillStyle = 'white';
      if (mobile) {
        const rows = [[0, 1], [2, 3]];
        rows.forEach((indices, row) => {
          ctx.font = `${row === 0 ? 155 : 100}px monospace`;
          const text = indices.map(i => words[i]).join(' '); let x = (1500 - ctx.measureText(text).width) / 2;
          for (const index of indices) { const w = ctx.measureText(words[index]).width; ctx.fillText(words[index], x, 105 + row * 200); boxes.push({ left: x, right: x + w, top: row * 200, bottom: (row + 1) * 200, part: index }); x += w + ctx.measureText(' ').width; }
        });
      } else {
        ctx.font = '80px monospace'; let x = (1500 - ctx.measureText(words.join(' ')).width) / 2;
        words.forEach((word, index) => { const w = ctx.measureText(word).width; ctx.fillText(word, x, 210); boxes.push({ left: x, right: x + w, top: 0, bottom: 420, part: index }); x += w + ctx.measureText(' ').width; });
      }
      const pixels = ctx.getImageData(0, 0, 1500, 420).data;
      const targets: { x: number; y: number; part: number }[] = [];
      const scale = width * (mobile ? .96 : .87) / 1500;
      for (let y = 0; y < 420; y += mobile ? 4 : 3) for (let x = 0; x < 1500; x += mobile ? 4 : 3) {
        if (pixels[(y * 1500 + x) * 4 + 3] > 100) targets.push({ x: width / 2 + (x - 750) * scale, y: height * (mobile ? .51 : .545) + (y - 210) * scale, part: boxes.find(b => x >= b.left && x <= b.right && y >= b.top && y <= b.bottom)?.part ?? 0 });
      }
      grains.removeParticles(); dots = [];
      const count = Math.max(targets.length, mobile ? 4200 : 8000);
      for (let i = 0; i < count; i++) {
        const panel = panels[i % panels.length]; const glyph = panel.glyphs[Math.floor(i * 1.618) % Math.max(1, panel.glyphs.length)] ?? { x: 0, y: 0 };
        const target = targets[i % targets.length]; const seed = ((i * 2654435761) >>> 0) / 4294967296;
        const particle = new Particle({ texture: Texture.WHITE, x: 0, y: 0, scaleX: 1.8, scaleY: 1.8, tint: 0xa3c9ad, alpha: 0 });
        grains.addParticle(particle); dots.push({ particle, sx: panel.x + glyph.x, sy: panel.y + glyph.y, sz: panel.z + .02, tx: target.x, ty: target.y, part: target.part, seed, angle: seed * Math.PI * 2 });
      }
    }
    syntaxTargets();
    const projection = new T.Matrix4();
    let previous = performance.now(), clock = 0;
    const resize = new ResizeObserver(() => {
      const nextWidth = viewport.clientWidth, nextHeight = viewport.clientHeight;
      if (nextWidth === width && nextHeight === height) return;
      width = nextWidth; height = nextHeight; camera.aspect = width / height; camera.updateProjectionMatrix();
      renderer.setSize(width, height); app.renderer.resize(width, height); syntaxTargets(); onScroll();
    }); resize.observe(viewport);
    function frame(now: number) {
      if (disposed) return;
      raf = requestAnimationFrame(frame);
      const dt = Math.min(.05, (now - previous) / 1000); previous = now;
      if (document.hidden) return;
      if (Math.abs(progress - target) < .00001 && target > .95 && lastScroll === scrollY) return;
      lastScroll = scrollY;
      progress += (target - progress) * (reduced ? 1 : 1 - Math.exp(-dt * 11));
      if (!reduced) clock += dt;
      const p = progress; dom(p);
      if (p >= .99) return;
      const mobile = width < 700;
      const tunnelProgress = clamp(p / .475);
      // Cubic travel: the same wheel distance covers progressively more space.
      const travel = reduced ? 105 : 235 * Math.pow(tunnelProgress, 2.8);
      const center = reduced ? 1 : smooth(.08, .43, p);
      camera.fov = reduced ? 43 : mix(43, 66, Math.pow(tunnelProgress, 2));
      camera.position.set(mix(mobile ? -1.3 : -6.3, .2, center), mix(mobile ? 3.8 : 2.2, .35, center), 15 - travel);
      camera.lookAt(mix(mobile ? 1 : 2.1, 0, center), mobile ? -.3 : 0, camera.position.z - 30);
      camera.updateProjectionMatrix(); camera.updateMatrixWorld();
      projection.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse);
      if (p < .55) {
        shaderTime.value = travel * .8 + clock * .2;
        const speed = reduced ? 0 : Math.pow(tunnelProgress, 2);
        streakMaterial.opacity = .06 + speed * .32;
        streakSeeds.forEach((s, i) => {
          const z = s.z + 6; const n = i * 6;
          streakPositions[n] = streakPositions[n + 3] = s.x;
          streakPositions[n + 1] = streakPositions[n + 4] = s.y;
          streakPositions[n + 2] = z; streakPositions[n + 5] = z - .2 - speed * 7;
        });
        streakGeometry.attributes.position.needsUpdate = true;
        pipeline.render();
      }
      if (p > .405 && p < .95) {
        const morph = reduced ? (p > .56 ? 1 : 0) : smooth(.44, .635, p);
        const m = projection.elements; const open = reduced ? (p > .83 ? 1 : 0) : smooth(.79, .94, p);
        let selectedX = 0, selectedN = 0;
        for (let i = 0; i < dots.length; i++) {
          const d = dots[i], x = d.sx, y = d.sy, z = d.sz;
          const w = m[3] * x + m[7] * y + m[11] * z + m[15];
          let sourceX: number, sourceY: number;
          if (w > .2) {
            sourceX = clamp(((m[0] * x + m[4] * y + m[8] * z + m[12]) / w * .5 + .5) * width, -width, width * 2);
            sourceY = clamp((.5 - (m[1] * x + m[5] * y + m[9] * z + m[13]) / w * .5) * height, -height, height * 2);
          } else { sourceX = width / 2 + Math.cos(d.angle) * width * .82; sourceY = height / 2 + Math.sin(d.angle) * height * .82; }
          const arcX = width / 2 + Math.cos(d.angle + morph * 1.1) * width * (.2 + d.seed * .6);
          const arcY = height * .5 + Math.sin(d.angle + morph * 1.1) * height * (.2 + d.seed * .55);
          const inv = 1 - morph;
          let px = inv * inv * sourceX + 2 * inv * morph * arcX + morph * morph * d.tx;
          let py = inv * inv * sourceY + 2 * inv * morph * arcY + morph * morph * d.ty;
          if (!reduced && morph > .98 && open === 0) {
            const dx = px - pointer.x, dy = py - pointer.y, distance = Math.hypot(dx, dy);
            if (distance < 100 && distance > 0) { const force = (1 - distance / 100) * 22; px += dx / distance * force; py += dy / distance * force; }
          }
          // The syntax moves with the opening green field rather than disappearing on a cut.
          px += (d.tx < width / 2 ? -1 : 1) * open * width * .65;
          py += Math.sin(d.angle) * open * height * .2;
          d.particle.x = px; d.particle.y = py;
          d.particle.tint = d.part === selected && morph > .8 ? 0xe2d391 : 0xa3c9ad;
          d.particle.alpha = (w < .2 ? smooth(.035, .22, morph) : 1) * mix(.66, d.part === selected ? .92 : .64, morph);
          const size = mix(1.3, mobile ? 1.5 : 1.85, morph) + Math.sin(morph * Math.PI) * 2.8;
          d.particle.scaleX = d.particle.scaleY = size;
          if (d.part === selected) { selectedX += d.tx; selectedN++; }
        }
        connector.clear();
        if (!mobile && morph > .98 && open < .05 && selectedN) {
          const x = selectedX / selectedN, y = height * .545 + 60;
          connector.moveTo(x, y).lineTo(x, y + 18).bezierCurveTo(x, y + 95, width * .73, height * .72, width * .73, height * .77).stroke({ color: 0xd8ce91, width: 1, alpha: .35 });
          connector.circle(x, y, 2.5).fill({ color: 0xe2d391, alpha: .7 });
        }
        app.render();
      }
    }
    viewport.dataset.engine = renderer.backend.isWebGPUBackend ? 'WebGPU + PixiJS' : 'WebGL2 + PixiJS';
    raf = requestAnimationFrame(frame);
    addEventListener('pagehide', () => { disposed = true; cancelAnimationFrame(raf); resize.disconnect(); renderer.dispose(); app.destroy(true, { children: true, texture: false }); }, { once: true });
  } catch (error) {
    console.error(error);
    document.querySelector('.journey-status')!.textContent = 'The scene could not start. Skip to Linger to read about the app.';
    // Keep the ordinary page reachable even when a GPU is unavailable.
    const fallback = () => { progress = target; dom(progress); };
    addEventListener('scroll', fallback, { passive: true }); fallback();
  }
}
