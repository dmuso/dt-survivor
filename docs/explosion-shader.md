# Explosion Shader Improvements

We need to improve the visuals of the fireball explosion shader.

## Explosion Visual Stages

The following outlines the visual stages that will make up an explosion effect:

1. Initial Impact Hit
2. Dark Impact Hit
3. Billowing Fire
4. Dark Projectiles
5. Ash Floats
6. Billowing Fire Turns to Smoke
7. Smoke Dissipation

### Initial Impact Hit

The initial impact hit is a bright hit strike, in a similar visual style to a 5-pointed star. It's not strictly a star however, the points are randomly distributed in size and direction (see: tmp/explosion-example-01.png). This impact hit grows in scale quickly. When it reaches it's largest size, visual stage 2 "Dark Impact Hit" & "Billowing Fire" spawns and starts. We can't yet see the dark impact hit or billowing fire yet, because the initial impact hit is always closest to the camera. The initial impact hit then shrinks in size quickly before being despawned. The initial impact hit is the front most effect closest to the camera.

### Dark Impact hit

At the same time as billowing fire starts, so does the dark impact hit. Dark impact hit is similar in style to the initial impact hit. It shows dark spikes growing and moving out from the impact point. tmp/explosion-example-09.png shows an example most clearly of what they look like.

### Billowing Fire

Billowing fire begins at the same time of the dark impact hit. It's spawned behind everything else, impact hits are rendered above this effect. It's shaped as many fire coloured spheres, growing in size and moving out from the impact point. It grows in size much slower than the initial impact hit. Many spheres are spawned, gradually moving outwards and growing. See tmp/explosion-example-09.png

### Dark Projectiles

Once the initial impact hit has despawned, we have dark projectiles that move quickly outwards from the impact point. They slow in speed quite fast however after they clear some distance from the impact point. These are circular projectiles elongated in a thin oval shape. The longer length of the oval is aligned to the direction of travel.

### Ash Floats

The dark projectiles turn into an ash float where they seem to float in the air for a short period before being despawned. Still moving, just slowly.

### Billowing Fire Turns to Smoke

The Billowing fire spheres turn dark and becomes the smoke. As it loses it's fire/orange visual, it continues to grow in scale.

### Smoke Dissipation

The billowing fire to smoke dissipates as the final phase. When dissipating, a smaller sphere/circle starting at the bottom of the smoke sphere grows and moves up to taking over the entire shape of the smoke sphere. As the smaller sphere grows, it acts like a transparent mask, slowly making the smoke disappear. See tmp/explosion-example-14.png and tmp/explosion-example-15.png for an example of the change.
