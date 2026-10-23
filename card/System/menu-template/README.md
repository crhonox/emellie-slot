# Tema del menú de Emellie Slot

El **menú principal** (Recents, Favorites, Collections, Games, Settings) y el **grid de
consolas** se dibujan con un tema: íconos, marco de selección, fuente, colores y cuántas
columnas y filas tiene el grid. Esta carpeta es una plantilla con el tema que trae Emellie Slot,
para que hagas el tuyo.

El carrusel de carts dentro de cada consola **no** usa este tema; sigue siendo el de omega-zero.

## Cómo activarlo

1. Copia esta carpeta (`System/menu-template`) a la raíz de la tarjeta con el nombre **`Menu`**,
   al lado de `Games`, `Saves` y `Config`:

   ```
   /Menu/
     menu.ini
     frame.png
     icons/gba.png ...
     720x720/ ...
   ```

2. Cambia lo que quieras: reemplaza PNGs, edita `menu.ini`.
3. Reinicia Emellie Slot. El tema se lee **una sola vez al arrancar**.

`System/` se reemplaza en cada actualización; `Menu/` no, así que tu tema se queda.

- **Volver al tema por defecto:** borra o renombra `Menu`.
- **Archivo que falta:** si a tu `Menu` le falta un archivo, se usa el que trae Emellie Slot.
  Puedes dejar solo lo que cambiaste.

## Resoluciones

Cada archivo se busca primero en la carpeta de la resolución de la consola y después en `Menu/`
mismo:

| Consola | Pantalla | Busca en |
|---|---|---|
| RG35XX SP (y 4:3 en general) | 640x480 | `Menu/640x480/`, luego `Menu/` |
| Pantallas anchas | 720x480 | `Menu/720x480/`, luego `Menu/` |
| RG CubeXX | 720x720 | `Menu/720x720/`, luego `Menu/` |

En la plantilla, la raíz tiene el set de 640x480 (que también sirve para 720x480) y `720x720/`
el set más grande de la CubeXX, igual que spruce. `menu.ini` también se combina: lo que pongas
en `720x720/menu.ini` gana sobre `menu.ini` en esa pantalla.

## Archivos

Todos son PNG con transparencia (RGBA). Se dibujan a su tamaño real, sin escalar.

| Archivo | Qué es | Tamaño en la plantilla |
|---|---|---|
| `icons/<consola>.png` | Ícono de la consola en el grid | 120x130 (134x146 en 720x720) |
| `icons/sel/<consola>.png` | El mismo ícono cuando está elegido | igual |
| `ic-<entrada>-n.png` | Ícono de una entrada del menú principal | 100x140 (112x156) |
| `ic-<entrada>-f.png` | El mismo ícono cuando está elegido | igual |
| `frame.png` | Marco detrás de la consola elegida en el grid | 154x170 (173x190) |
| `background.png` | Fondo de pantalla completa del menú y del grid (opcional) | 640x480 / 720x480 / 720x720 |
| `font.ttf` | Fuente de los nombres bajo los íconos | — |
| `menu.ini` | Ajustes (ver abajo) | — |

**Nombres de consola:** `gba`, `gb`, `gbc`, `sfc` (SNES), `ps` (PlayStation), `nds`, `ports`.

**Nombres de entrada:** `recent`, `favorite`, `collection`, `game`, `setting`.

Si un ícono elegido (`sel/` o `-f`) falta, se usa el normal.

### El fondo

Sin `background.png` el menú se dibuja sobre el **fondo de omega-zero**: el patrón o el
wallpaper que elegiste en Settings > Background. Así viene por defecto.

Para usar el fondo de spruce, renombra `background-spruce.png` a `background.png`. Hay uno para
cada resolución: la raíz es 640x480, y `720x480/` y `720x720/` tienen los suyos. También puedes
poner tu propia imagen del tamaño de la pantalla. El fondo cubre la pantalla entera; las barras
de arriba (hora, batería) y de abajo (botones) se dibujan encima.

### La fuente

`font.ttf` es la fuente de spruce (`nunwen.ttf`), recortada a letras latinas para que pese poco. Cualquier
`.ttf` sirve. Una fuente muy grande (por ejemplo, con caracteres asiáticos) tarda en cargar al
arrancar.

## menu.ini

```ini
grid_columns = 4          # columnas del grid (1 a 8)
grid_rows = 2             # filas del grid (1 a 6); con más consolas pasa de página
grid_text_size = 18       # tamaño de los nombres en el grid, en píxeles
home_text_size = 21       # tamaño de los nombres en el menú principal
text_colour = #7C6F64     # color de los nombres
text_lit_colour = #FBF1C7 # color del nombre elegido
names = yes               # yes/no: escribir el nombre bajo cada ícono
frame = none              # para no dibujar marco detrás de la consola elegida
```

En 720x720 los valores por defecto son `grid_rows = 3`, `grid_text_size = 20` y
`home_text_size = 24`.

## Cómo se acomoda todo

Así puedes calcular de qué tamaño hacer tus PNG.

**Grid de consolas:**
- Usa el espacio entre la barra de arriba (36 px) y la de abajo (30 px), con 10 px de margen a
  cada lado.
- Ese espacio se divide en `grid_columns` x `grid_rows` celdas iguales. Por ejemplo, en 640x480
  con 4x2, cada celda mide 155x207 px.
- En cada celda, el ícono va centrado y subido media línea de texto. El nombre va debajo, en la
  parte baja de la celda.
- El marco se centra en la celda.
- Las consolas sin juegos no aparecen.

**Menú principal:**
- Las 5 entradas van en una fila, cada una en su quinta parte del ancho.
- El ícono va centrado en la altura disponible.
- El nombre va 14 px debajo del ícono (contando el alto de un ícono de 140 px; de 156 px en
  720x720).

Íconos más grandes que la celda se salen de ella. Si haces íconos más grandes, baja las columnas
o filas.

## Si algo no se ve

Revisa `emellie-slot.log` en la tarjeta. Un PNG que no se puede leer o una fuente inválida dejan
una línea `menu theme: ...`, y en su lugar se usa el archivo por defecto.

## Créditos

Los íconos, el marco, los fondos y la fuente de la plantilla son del tema SPRUCE de spruceOS
(tenlevels y el equipo de spruceUI), con licencia CC BY-NC 4.0: uso no comercial, con crédito.
El texto completo está en `System/licenses/spruce-theme.txt`.
