# The Home room

An original lofi night room: a person asleep at a desk, your devices on the furniture, the bolt as a trophy on the shelf. `scene.js` draws it, `layout.mjs` holds its numbers, `build.mjs` renders both into `build/` as images both apps use, and `room.json` says where everything goes. After editing any of them, run `npm run room` in `desktop/` and commit `build/`.

Everything is placed by one camera in centimetres (`project` in `layout.mjs`, and `P3` in `scene.js`): eye height 146 cm, the back wall 480 cm away. Furniture reaches the floor because the floor line is calculated.

Devices have eight fixed places, in the order they fill. A device is drawn in place for each one, turned toward the room, and never resized. `build.mjs` refuses to build if a footprint hangs off its surface or two footprints overlap, and each screen only offers the places where a device and its name tag land on clear screen (`roomSlots` in `room.ts` and `RoomModel.kt`). A device with no place left waits in the Devices list.

Three crops of the same room: `tall` for phones, `mid` for upright tablets and foldables, `wide` for everything else. Rotating a screen just changes crop.

The window's frame, curtains and sill are also a layer of their own (`front-*`), drawn over the rain so it stays behind the glass.

Solid colours only, no gradients: the lamp light is hard-edged bands made with colour filters.
