// Draws a replayed frame: the stage's surfaces inside its blast zone, each
// fighter's hurt volumes (gold, white while invincible, dashed while
// intangible), its active strikes in red and its projectiles, and each
// fighter's damage and stocks along the bottom.
import type { ReplayScene } from "./replays";

const SLOT_COLORS = ["#e74c3c", "#4f8fe6", "#f0c93a", "#48c774"];
const PART_COLORS = ["rgba(230, 160, 40, 0.55)", "rgba(255, 255, 255, 0.7)", "rgba(120, 170, 255, 0.45)"];

type Capsule = { x1: number; z1: number; x2: number; z2: number; radius: number };

export function drawScene(canvas: HTMLCanvasElement, scene: ReplayScene, names: readonly string[]): void {
  const context = canvas.getContext("2d");
  if (context === null) return;
  const { width, height } = canvas;
  const hud = 56;
  const { left, right, bottom, top } = scene.blast;
  const scale = Math.min(width / (right - left), (height - hud) / (top - bottom));
  const originX = (width - (right - left) * scale) / 2 - left * scale;
  const originY = (height - hud + (top - bottom) * scale) / 2 + bottom * scale;
  const x = (worldX: number) => originX + worldX * scale;
  const y = (worldZ: number) => originY - worldZ * scale;

  context.fillStyle = "#15110d";
  context.fillRect(0, 0, width, height);
  context.strokeStyle = "#3d3125";
  context.lineWidth = 1;
  context.strokeRect(x(left), y(top), (right - left) * scale, (top - bottom) * scale);

  context.lineCap = "round";
  context.strokeStyle = "#8c7a62";
  context.lineWidth = 3;
  for (const surface of scene.surfaces) {
    context.beginPath();
    context.moveTo(x(surface.left), y(surface.z));
    context.lineTo(x(surface.right), y(surface.z));
    context.stroke();
  }

  const capsule = (shape: Capsule, color: string, dashed = false) => {
    context.strokeStyle = color;
    context.lineWidth = Math.max(1, shape.radius * 2 * scale);
    context.setLineDash(dashed ? [4, 4] : []);
    context.beginPath();
    context.moveTo(x(shape.x1), y(shape.z1));
    context.lineTo(x(shape.x2), y(shape.z2));
    context.stroke();
  };
  for (const fighter of scene.fighters) {
    if (fighter.out) continue;
    for (const part of fighter.parts) capsule(part, PART_COLORS[part.state] ?? PART_COLORS[0]!, part.state === 2);
    context.setLineDash([]);
    for (const strike of fighter.strikes) capsule(strike, "rgba(231, 76, 60, 0.75)");
    context.fillStyle = SLOT_COLORS[fighter.slot] ?? "#fff";
    for (const projectile of fighter.projectiles) {
      context.beginPath();
      context.arc(x(projectile.x), y(projectile.z), 4, 0, Math.PI * 2);
      context.fill();
    }
    context.beginPath();
    context.arc(x(fighter.x), y(fighter.z), 3, 0, Math.PI * 2);
    context.fill();
  }
  context.setLineDash([]);

  const column = width / Math.max(1, scene.fighters.length);
  context.textAlign = "center";
  scene.fighters.forEach((fighter, index) => {
    const centre = column * index + column / 2;
    context.fillStyle = SLOT_COLORS[fighter.slot] ?? "#fff";
    context.font = "bold 13px system-ui, sans-serif";
    context.fillText(names[fighter.slot] ?? `P${fighter.slot + 1}`, centre, height - hud + 18);
    context.fillStyle = "#f3eadb";
    context.font = "bold 20px system-ui, sans-serif";
    context.fillText(fighter.out ? "—" : `${Math.round(fighter.damage)}%`, centre, height - hud + 40);
    context.font = "12px system-ui, sans-serif";
    context.fillStyle = "#b3a48d";
    context.fillText("●".repeat(Math.max(0, fighter.stocks)), centre, height - 4);
  });
}
