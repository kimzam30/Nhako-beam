// The 3D stage (three.js): the voxel mascot on its floor, and on Home your devices around it,
// ripples while it searches, an arc to each device that turns up, and sparks while files move.
// Loaded only when a stage is shown (`createStage` is imported lazily), so the first paint
// doesn't wait for it. It draws while something moves and the window is in sight; with reduced
// motion it draws still poses, only when something changes. motion-spec.md section 4.18.

import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import glbData from '../../../../design/mascot/build/mascot.glb?inline';
import mascot from '../../../../design/mascot/build/mascot.json';
import { ALL_FACE_PARTS, FACE_PARTS, MascotMotion, type Look, type MascotState, type Pose } from './pose';
import { splitAlpha, type Camera, type Vec3 } from './view';

export interface StageDevice {
  id: string;
  kind: 'laptop' | 'desktop' | 'phone' | 'tablet';
  /** Where it stands on the floor. */
  at: Vec3;
  /** Blue screen when it can be reached; grey and faded when not. */
  reachable: boolean;
  /** Raised a little: under the pointer, or files dragged near it. */
  lifted: boolean;
}

export interface StageOptions {
  camera: Camera;
  /** Home: the floor with its guide rings, and room for devices. Elsewhere a small shadow only. */
  home: boolean;
  /** The bolt's size (the model is 3 units high). */
  mascotScale: number;
  /** Two bolts that hop towards each other (pairing). */
  pair?: boolean;
  /** Drop in from above, land, blink (the launch intro). */
  intro?: boolean;
  reduced: boolean;
}

export interface Stage {
  setMood(state: MascotState): void;
  setLook(look: Look): void;
  setDevices(list: StageDevice[]): void;
  /** Sparks run between the bolt and this device while files move; null: none. */
  setTransferDevice(id: string | null): void;
  /** A lightning arc from the bolt to a device that just turned up. */
  arc(id: string): void;
  setReduced(reduced: boolean): void;
  /** Re-reads the theme's colors (floor, rings). */
  retheme(): void;
  /** Stops drawing while something covers it (a sheet), and starts again after. */
  setPaused(paused: boolean): void;
  resize(width: number, height: number): void;
  destroy(): void;
}

const css = (name: string, fallback: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
const VOXEL = 1 / 16;

let modelPromise: Promise<THREE.Group> | null = null;
/** The model, parsed once; each stage gets its own copy. */
function loadModel(): Promise<THREE.Group> {
  modelPromise ??= new Promise((resolve, reject) => {
    const base64 = glbData.slice(glbData.indexOf(',') + 1);
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    new GLTFLoader().parse(bytes.buffer, '', (g) => resolve(g.scene as THREE.Group), reject);
  });
  return modelPromise;
}

/** A device from its boxes in mascot.json, standing on its own origin. */
function buildDevice(kind: StageDevice['kind']): { group: THREE.Group; screens: THREE.MeshStandardMaterial[]; mats: THREE.MeshStandardMaterial[] } {
  const group = new THREE.Group();
  const colors = mascot.deviceColors;
  const screens: THREE.MeshStandardMaterial[] = [];
  const mats: THREE.MeshStandardMaterial[] = [];
  for (const b of mascot.devices[kind]) {
    const role = b.role as 'case' | 'key' | 'screen';
    const mat = new THREE.MeshStandardMaterial({ color: colors[role], roughness: role === 'screen' ? 0.4 : 0.85, transparent: true });
    if (role === 'screen') {
      mat.emissive = new THREE.Color(colors.screen);
      mat.emissiveIntensity = 0.35;
      screens.push(mat);
    }
    mats.push(mat);
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(b.size[0]! * VOXEL, b.size[1]! * VOXEL, b.size[2]! * VOXEL), mat);
    mesh.position.set(b.at[0]! * VOXEL, b.at[1]! * VOXEL, b.at[2]! * VOXEL);
    if ('tilt' in b && typeof b.tilt === 'number') mesh.rotation.x = -(b.tilt * Math.PI) / 180;
    mesh.castShadow = true;
    group.add(mesh);
  }
  return { group, screens, mats };
}

/** A spring towards a target, for lifts and pops. */
class Spring {
  v = 0;
  constructor(public x: number, public target: number, private k = 260, private c = 22) {}
  step(dt: number) {
    const a = this.k * (this.target - this.x) - this.c * this.v;
    this.v += a * dt;
    this.x += this.v * dt;
    return Math.abs(this.target - this.x) > 0.001 || Math.abs(this.v) > 0.001;
  }
}

export async function createStage(canvas: HTMLCanvasElement, opts: StageOptions): Promise<Stage> {
  const source = await loadModel();
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(opts.camera.fovDeg, 1, 0.1, 100);
  camera.position.set(...opts.camera.eye);
  camera.lookAt(...opts.camera.target);

  // One key light from the upper left, a soft sky fill, a shadow on the floor.
  const hemi = new THREE.HemisphereLight(0xffffff, 0xc9d3e3, 1.35);
  const key = new THREE.DirectionalLight(0xffffff, 1.9);
  key.position.set(-4, 8, 6);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  key.shadow.radius = 6;
  Object.assign(key.shadow.camera, { left: -6, right: 6, top: 6, bottom: -6 });
  scene.add(hemi, key);

  // The floor.
  const R = opts.home ? mascot.stage.floorRadius : 1.6;
  const floorMat = new THREE.MeshBasicMaterial({ color: css('--color-fill', '#ECEEF2'), transparent: true, opacity: opts.home ? 1 : 0 });
  const floor = new THREE.Mesh(new THREE.CircleGeometry(R, 72), floorMat);
  floor.rotation.x = -Math.PI / 2;
  const shadow = new THREE.Mesh(new THREE.CircleGeometry(R, 72), new THREE.ShadowMaterial({ opacity: opts.home ? 0.16 : 0.2 }));
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.y = 0.002;
  shadow.receiveShadow = true;
  scene.add(floor, shadow);

  // Home's guide rings (the radar's), and the ripples that run out while it searches.
  // The separator token carries its own alpha, as on Android.
  const ringTint = () => splitAlpha(css('--color-separator', '#1214181A'));
  const ringMat = new THREE.MeshBasicMaterial({ color: ringTint().color, transparent: true, opacity: ringTint().alpha });
  const rippleMat = new THREE.MeshBasicMaterial({ color: css('--color-accent', '#1A6FE0'), transparent: true, opacity: 0 });
  const ripples: THREE.Mesh[] = [];
  if (opts.home) {
    for (const f of [0.66, 1]) {
      const ring = new THREE.Mesh(new THREE.RingGeometry(mascot.stage.slotReach * f - 0.012, mascot.stage.slotReach * f + 0.012, 96), ringMat);
      ring.rotation.x = -Math.PI / 2;
      ring.position.y = 0.004;
      scene.add(ring);
    }
    for (let i = 0; i < 2; i++) {
      const r = new THREE.Mesh(new THREE.RingGeometry(0.97, 1, 96), rippleMat.clone());
      r.rotation.x = -Math.PI / 2;
      r.position.y = 0.006;
      r.visible = false;
      scene.add(r);
      ripples.push(r);
    }
  }

  // The bolt (two for pairing).
  const rigs: { rig: THREE.Group; parts: Record<string, THREE.Object3D>; motion: MascotMotion; zzzBase: number; eyeBase: Record<string, number> }[] = [];
  for (let i = 0; i < (opts.pair ? 2 : 1); i++) {
    const model = source.clone(true);
    const parts: Record<string, THREE.Object3D> = {};
    model.traverse((o) => {
      if ((o as THREE.Mesh).isMesh) {
        const mesh = o as THREE.Mesh;
        mesh.castShadow = true;
        if (/^sparkle|^zzz/.test(o.name)) mesh.material = (mesh.material as THREE.Material).clone();
      }
      if (o.name) parts[o.name] = o;
    });
    for (const p of Object.values(parts)) if (p.userData.hidden) p.visible = false;
    const rig = new THREE.Group();
    rig.add(model);
    rig.scale.setScalar(opts.mascotScale);
    if (opts.pair) rig.position.x = i === 0 ? -1.15 : 1.15;
    scene.add(rig);
    rigs.push({
      rig,
      parts,
      motion: new MascotMotion(),
      zzzBase: parts.zzz?.position.y ?? 0,
      eyeBase: { eyeL: parts.eyeL?.position.x ?? 0, eyeR: parts.eyeR?.position.x ?? 0 },
    });
  }
  const levels = { hemi: hemi.intensity, key: key.intensity };

  // Devices.
  type Shown = { d: StageDevice; group: THREE.Group; screens: THREE.MeshStandardMaterial[]; mats: THREE.MeshStandardMaterial[]; pop: Spring; lift: Spring; leaving: boolean };
  const devices = new Map<string, Shown>();
  const screenAway = new THREE.Color(mascot.deviceColors.screenAway);
  const screenOn = new THREE.Color(mascot.deviceColors.screen);

  // Sparks and arcs: small yellow voxels, reused.
  const sparkGeo = new THREE.BoxGeometry(0.1, 0.1, 0.1);
  type Spark = { m: THREE.Mesh; born: number; life: number; from: THREE.Vector3; to: THREE.Vector3; lift: number; vel: THREE.Vector3 | null };
  const sparks: Spark[] = Array.from({ length: 64 }, () => {
    const m = new THREE.Mesh(sparkGeo, new THREE.MeshBasicMaterial({ color: 0xffd23f, transparent: true }));
    m.visible = false;
    scene.add(m);
    return { m, born: 0, life: 0, from: new THREE.Vector3(), to: new THREE.Vector3(), lift: 0, vel: null };
  });
  let sparkNext = 0;
  const emit = (from: THREE.Vector3, to: THREE.Vector3, life: number, lift: number, now: number) => {
    const s = sparks[sparkNext++ % sparks.length]!;
    s.from.copy(from);
    s.to.copy(to);
    s.life = life;
    s.lift = lift;
    s.born = now;
    s.vel = null;
    s.m.visible = true;
    return s;
  };
  type Arc = { group: THREE.Group; born: number };
  const arcs: Arc[] = [];
  const arcMat = new THREE.MeshBasicMaterial({ color: 0xffd23f, transparent: true });

  let reduced = opts.reduced;
  let mood: MascotState = 'idle';
  let look: Look = null;
  let transferDevice: string | null = null;
  let raf = 0;
  let last = performance.now();
  const born = performance.now();
  let visible = true;
  let paused = false;
  let lastDrawn = 0;
  const io = new IntersectionObserver(([e]) => {
    visible = !!e?.isIntersecting;
    wake();
  });
  io.observe(canvas);
  const onVisibility = () => wake();
  document.addEventListener('visibilitychange', onVisibility);

  for (const r of rigs) r.motion.set(opts.pair ? 'happy' : 'idle', born);

  /** The top of the bolt's head, where arcs and sparks leave from. */
  const boltTop = () => new THREE.Vector3(0, 3 * opts.mascotScale * 0.98, 0);

  function applyPose(r: (typeof rigs)[number], p: Pose, intro: number) {
    const { rig, parts } = r;
    const s = opts.mascotScale;
    rig.position.y = p.y * s + intro;
    rig.rotation.set(p.rx, p.ry, p.rz);
    rig.scale.set(s * p.sx, s * p.sy, s * p.sx);
    const shown = new Set(FACE_PARTS[p.face]);
    for (const n of ALL_FACE_PARTS) if (parts[n]) parts[n].visible = shown.has(n);
    for (const n of ['eyeL', 'eyeR']) {
      const e = parts[n];
      if (!e) continue;
      e.scale.y = p.eyeOpen;
      e.position.x = r.eyeBase[n]! + p.eyeShift;
    }
    if (parts.crack) parts.crack.visible = p.crack;
    const z = parts.zzz as THREE.Mesh | undefined;
    if (z) {
      z.visible = !!p.zzz;
      if (p.zzz) {
        z.position.y = r.zzzBase + p.zzz.rise * 6;
        const m = z.material as THREE.MeshStandardMaterial;
        m.transparent = true;
        m.opacity = p.zzz.opacity;
      }
    }
    p.sparkles.forEach((v, i) => {
      const sp = parts[`sparkle${i + 1}`];
      if (!sp) return;
      sp.visible = v > 0;
      sp.scale.setScalar(Math.max(0.001, v));
    });
    hemi.intensity = levels.hemi * p.light;
    key.intensity = levels.key * p.light;
  }

  /** The intro: falls from above with gravity, lands with a squash. Returns the lift. */
  function introLift(now: number, r: (typeof rigs)[number]) {
    if (!opts.intro || reduced) return 0;
    const t = (now - born) / 1000;
    const fall = 0.42;
    if (t < fall) {
      const a = t / fall;
      return (1 - a * a) * 3.2;
    }
    const land = (t - fall) / 0.22;
    if (land < 1) {
      const sq = Math.sin(land * Math.PI);
      r.rig.scale.y *= 1 - sq * 0.18;
      r.rig.scale.x *= 1 + sq * 0.09;
      r.rig.scale.z *= 1 + sq * 0.09;
    }
    return 0;
  }

  // Calm states (a breath, a blink) look the same at 30 frames a second, for half the work.
  const CALM = new Set<MascotState>(['idle', 'asleep', 'offline']);

  function frame(now: number) {
    raf = 0;
    if (!reduced && CALM.has(mood) && !sparks.some((s) => s.m.visible) && !arcs.length && now - lastDrawn < 31) {
      wake();
      return;
    }
    lastDrawn = now;
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    let moving = !reduced;

    rigs.forEach((r, i) => {
      if (reduced) applyPose(r, r.motion.still(look), 0);
      else {
        const p = r.motion.step(now, dt, opts.pair ? { x: i === 0 ? 0.8 : -0.8, y: 0 } : look);
        applyPose(r, p, 0);
        const lift = introLift(now, r);
        r.rig.position.y += lift;
        if (opts.pair) {
          // The two bolts lean in towards each other.
          r.rig.rotation.z += (i === 0 ? -1 : 1) * 0.08;
        }
        const target = transferDevice ? devices.get(transferDevice) : undefined;
        if (p.sparks && target) {
          const at = target.group.position.clone().add(new THREE.Vector3(0, 0.45, 0));
          const from = boltTop();
          emit(p.sparks === 'out' ? from : at, p.sparks === 'out' ? at : from, 0.85, 0.7 + Math.random() * 0.4, now);
        }
        if (p.burst) {
          const c = boltTop();
          for (let k = 0; k < 18; k++) {
            const a = (k / 18) * Math.PI * 2;
            const s = emit(c, c, 0.8, 0, now);
            s.vel = new THREE.Vector3(Math.cos(a) * 2.2, Math.sin(a) * 2.2 + 1, (Math.random() - 0.5) * 1.4);
          }
        }
      }
    });

    for (const [id, s] of devices) {
      const a = s.pop.step(dt);
      const b = s.lift.step(dt);
      if (a || b) moving = true;
      s.group.scale.setScalar(Math.max(0.001, s.pop.x) * mascot.stage.deviceScale);
      s.group.position.y = s.lift.x;
      if (s.leaving && s.pop.x < 0.02) {
        scene.remove(s.group);
        devices.delete(id);
      }
    }

    // Ripples run out from the bolt while searching, or slowly while nobody's around.
    const ripplesOn = opts.home && !reduced && (mood === 'looking' || mood === 'asleep') && devices.size === 0;
    ripples.forEach((r, i) => {
      r.visible = ripplesOn;
      if (!ripplesOn) return;
      const period = mood === 'looking' ? 2.4 : 3.8;
      const a = (((now - born) / 1000 / period + i / 2) % 1 + 1) % 1;
      const rad = 0.7 + a * (mascot.stage.slotReach + 0.3);
      r.scale.setScalar(rad);
      (r.material as THREE.MeshBasicMaterial).opacity = 0.5 * (1 - a);
    });

    for (const s of sparks) {
      if (!s.m.visible) continue;
      moving = true;
      const a = (now - s.born) / 1000 / s.life;
      if (a >= 1) {
        s.m.visible = false;
        continue;
      }
      if (s.vel) {
        const tt = (now - s.born) / 1000;
        s.m.position.set(s.from.x + s.vel.x * tt, s.from.y + s.vel.y * tt - 4.5 * tt * tt, s.from.z + s.vel.z * tt);
      } else {
        const e = a < 0.5 ? 4 * a * a * a : 1 - (-2 * a + 2) ** 3 / 2;
        s.m.position.lerpVectors(s.from, s.to, e);
        s.m.position.y += Math.sin(a * Math.PI) * s.lift;
      }
      s.m.rotation.set(a * 6, a * 4, 0);
      (s.m.material as THREE.MeshBasicMaterial).opacity = a > 0.7 ? (1 - a) / 0.3 : 1;
    }

    for (let i = arcs.length - 1; i >= 0; i--) {
      const arc = arcs[i]!;
      const a = (now - arc.born) / 420;
      if (a >= 1) {
        scene.remove(arc.group);
        arcs.splice(i, 1);
        continue;
      }
      moving = true;
      // Strikes in over the first third, then fades.
      arc.group.children.forEach((c, k) => (c.visible = k / arc.group.children.length < a * 3));
      arcMat.opacity = a < 0.5 ? 1 : 1 - (a - 0.5) * 2;
    }

    renderer.render(scene, camera);
    if (moving || ripplesOn) wake();
  }

  function wake() {
    if (raf || !visible || paused || document.hidden) return;
    raf = requestAnimationFrame(frame);
  }

  function setDevices(list: StageDevice[]) {
    const ids = new Set(list.map((d) => d.id));
    for (const [id, s] of devices) {
      if (!ids.has(id) && !s.leaving) {
        s.leaving = true;
        s.pop.target = 0;
        if (reduced) s.pop.x = 0;
      }
    }
    for (const d of list) {
      let s = devices.get(d.id);
      if (!s || s.leaving) {
        if (s) scene.remove(s.group);
        const built = buildDevice(d.kind);
        // Devices already there when the stage first shows stand there; later ones pop in.
        s = { d, ...built, pop: new Spring(reduced || !lastDrawn ? 1 : 0, 1, 500, 13), lift: new Spring(0, 0), leaving: false };
        devices.set(d.id, s);
        scene.add(s.group);
      }
      s.d = d;
      s.group.position.set(d.at[0], s.lift.x, d.at[2]);
      // Each device faces you (the camera), so its screen shows.
      s.group.rotation.y = Math.atan2(opts.camera.eye[0] - d.at[0], opts.camera.eye[2] - d.at[2]);
      for (const m of s.screens) {
        m.color.copy(d.reachable ? screenOn : screenAway);
        m.emissive.copy(d.reachable ? screenOn : screenAway);
        m.emissiveIntensity = d.reachable ? 0.35 : 0.05;
      }
      for (const m of s.mats) m.opacity = d.reachable ? 1 : 0.55;
      s.lift.target = d.lifted ? 0.18 : 0;
      if (reduced) s.lift.x = s.lift.target;
    }
    wake();
  }

  function arc(id: string) {
    const s = devices.get(id);
    if (!s || reduced) return;
    const from = boltTop();
    const to = new THREE.Vector3(s.d.at[0], 0.5, s.d.at[2]);
    const group = new THREE.Group();
    const n = 9;
    let prev = from.clone();
    for (let i = 1; i <= n; i++) {
      const a = i / n;
      const p = from.clone().lerp(to, a);
      p.y += Math.sin(a * Math.PI) * 0.6;
      if (i < n) {
        p.x += (Math.random() - 0.5) * 0.35;
        p.y += (Math.random() - 0.5) * 0.25;
      }
      const len = prev.distanceTo(p);
      const seg = new THREE.Mesh(new THREE.BoxGeometry(0.05, 0.05, len), arcMat);
      seg.position.copy(prev).lerp(p, 0.5);
      seg.lookAt(p);
      group.add(seg);
      prev = p;
    }
    scene.add(group);
    arcs.push({ group, born: performance.now() });
    wake();
  }

  const stage: Stage = {
    setMood(state) {
      if (state === mood) return;
      mood = state;
      const now = performance.now();
      if (!opts.pair) rigs.forEach((r) => r.motion.set(state, now));
      wake();
    },
    setLook(l) {
      look = l;
      wake();
    },
    setDevices,
    setTransferDevice(id) {
      transferDevice = id;
    },
    arc,
    setReduced(r) {
      reduced = r;
      wake();
    },
    retheme() {
      floorMat.color.set(css('--color-fill', '#ECEEF2'));
      const ring = ringTint();
      ringMat.color.set(ring.color);
      ringMat.opacity = ring.alpha;
      for (const r of ripples) (r.material as THREE.MeshBasicMaterial).color.set(css('--color-accent', '#1A6FE0'));
      wake();
    },
    setPaused(p) {
      paused = p;
      last = performance.now();
      wake();
    },
    resize(width, height) {
      if (!width || !height) return;
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      wake();
    },
    destroy() {
      cancelAnimationFrame(raf);
      io.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
      // The model's geometry is shared with the cached copy, so only the context goes.
      renderer.dispose();
      renderer.forceContextLoss();
    },
  };
  wake();
  return stage;
}
