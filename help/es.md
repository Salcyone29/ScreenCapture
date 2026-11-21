# Bienvenida
ScreenCapture captura la pantalla, anota las imágenes y graba vídeos. Casi todas las acciones se pueden hacer de varias formas: atajo de teclado, botón de Inicio, menú del área de notificación o ratón. Esta ayuda las presenta todas.
![accueil]
- **Inicio**: lanzar una captura y consultar el historial.
- **Ajustes**: carpeta, formato, atajos, conservación, idioma, actualizaciones.
- **Ayuda**: esta página. Tecla `F1` desde Inicio.
> ScreenCapture sigue disponible en el área de notificación (junto al reloj): la cruz de la ventana la oculta sin salir de la aplicación.
> En Mac, `Ctrl` corresponde a `⌘` y los atajos de captura se eligen en Ajustes.

# Hacer una captura
Seis tipos de captura:
- **Zona**: una parte de la pantalla, elegida con el ratón.
- **Pantalla completa**: todas las pantallas a la vez.
- **Ventana**: la ventana en primer plano.
- **Última zona**: la misma zona que la captura anterior, sin volver a dibujarla.
- **Diferida**: una zona, tras una cuenta atrás (para abrir un menú, por ejemplo).
- **Vídeo**: grabación de una zona, ver «Grabar un vídeo».
## Para lanzar una captura
- El **atajo de teclado**, desde cualquier aplicación: `Impr Pant` para una zona, `Ctrl + Impr Pant` para la pantalla completa, `Alt + Impr Pant` para una ventana, `Mayús + Impr Pant` para la última zona, `Ctrl + Mayús + Impr Pant` para el vídeo. Se cambian en Ajustes.
- Las **tarjetas de Inicio** (Zona, Pantalla completa, Ventana…).
- El **menú del área de notificación**: clic derecho en el icono de ScreenCapture.
- El **botón de la barra izquierda** (arriba), para una zona.
- La **línea de comandos**: `ScreenCapture.exe --cmd region` (ver «Para ir más lejos»).
## Elegir la zona
![selection]
- **Arrastrar** para dibujar la zona; al soltar se captura.
- **Ampliar hacia abajo** (Windows): después de dibujar la zona, girar la **rueda sin soltar el botón**. La selección deja paso a la página, que se desplaza con normalidad dentro de un marco fijo; todo lo que pasa por el marco se une. Soltar el botón para capturar.
- **Hacer clic** en una ventana para capturarla entera (se resalta al pasar el ratón).
- `Intro` captura la pantalla entera, `Esc` cancela.
- `C` copia el color bajo el cursor (código hexadecimal); la lupa muestra los píxeles ampliados.
- La barra de ayuda se aparta sola si tapa la zona que está seleccionando.
## Después de la captura
Según el ajuste «Después de la captura»:
- **Miniatura**: aparece una miniatura durante unos segundos. Al hacer clic se abre el editor.
- **Editor**: la imagen se abre directamente para anotarla.
- **Nada**: la captura solo se guarda y/o se copia.
La captura también puede copiarse automáticamente al portapapeles y guardarse en la carpeta de capturas (Ajustes).
## Captura con desplazamiento
Para capturar una página más alta que la pantalla (página web, documento…): tarjeta **Desplazamiento** de la página de inicio, o menú del área de notificación.
- Seleccionar la zona que se desplaza, preferiblemente sin la barra de desplazamiento.
- La página se desplaza sola hacia abajo y las imágenes se unen; la captura se detiene al final de la página, o con **Terminar** / `Esc`.
- Para desplazar usted mismo, mueva el ratón: el desplazamiento automático se detiene y la unión sigue su rueda (sin ir demasiado rápido), hasta **Terminar**.
- Los encabezados y pies de página fijos solo se conservan una vez.
> En Mac, el desplazamiento automático necesita el permiso **Accesibilidad** (Ajustes del Sistema > Privacidad y seguridad). Sin él, desplace usted mismo la página con la rueda o el trackpad y luego **Terminar**.

# Anotar en el editor
![editeur]
El editor se abre tras una captura, con un doble clic en una miniatura del historial, con el botón «Abrir una imagen» o arrastrando un archivo de imagen a la ventana.
## Las herramientas
Cada herramienta tiene una tecla (sin Ctrl):
- `V` Selección, `A` Flecha, `L` Línea, `R` Rectángulo, `E` Elipse, `P` Mano alzada.
- `H` Resaltador, `B` Desenfoque / pixelado, `M` Lupa.
- `T` Texto, `U` Bocadillo, `N` Numeración, `S` Sello.
- `C` Recortar, `X` Recortar una franja, `D` Mover una zona, `I` Cuentagotas.
Mantener `Mayús` al dibujar da ángulos de 45° (flechas, líneas) o formas regulares (cuadrado, círculo).
## Modificar una forma
- **Seleccionar**: herramienta Selección y clic en la forma. Los tiradores permiten cambiar su tamaño.
- **Mover**: arrastrar la forma, o las flechas del teclado (`Mayús` + flecha: 10 píxeles).
- **Moverse por la imagen ampliada**: mantener pulsado el **botón derecho** (o el central) y mover el ratón; la rueda también desplaza.
- **Bocadillos**: cuatro tipos (redondeado, rectangular, ovalado, pensamiento) en el panel derecho. El **tirador de la punta** se arrastra para orientarla o alargarla sin mover el bocadillo; «Punta predeterminada» la recoloca.
- **Duplicar**: `Ctrl + D`, o el botón «Duplicar» del panel derecho. Un número duplicado toma el número siguiente de la serie.
- **Modificar el contenido**: **doble clic** en un texto, bocadillo, número o sello (o botón «Modificar el contenido»). Para un número, escriba el nuevo valor (5, e, IV… según el estilo) y pulse `Intro`.
- **Color, grosor, tamaño del texto, relleno, sombra, discontinuo**: panel derecho, aplicados a la forma seleccionada, o a las siguientes formas si no hay nada seleccionado.
- **Traer al frente / enviar al fondo** y **eliminar** (`Supr`): botones del panel derecho.
- **Deshacer / rehacer**: `Ctrl + Z` / `Ctrl + Y`.
## Números y sellos
- **Numeración**: cada clic coloca el número siguiente. El panel derecho elige el estilo (1, 2, 3 · a, b, c · I, II, III…), el próximo valor, y puede volver a numerar todo en orden.
- **Sellos**: APROBADO, CONFIDENCIAL, BORRADOR… o su propio texto en el campo **Sello personalizado** (se recuerda). Un clic en la imagen coloca el sello; un doble clic en un sello colocado modifica su texto.
## Transformar la imagen
- **Recortar** (`C`): arrastrar la zona que se conserva y pulsar `Intro`.
- **Recortar una franja** (`X`): quita una franja horizontal o vertical y vuelve a unir las dos partes (unión recta o en zigzag).
- **Mover una zona** (`D`): seleccionar una parte de la imagen y arrastrarla; el lugar de origen se rellena con el color del fondo.
- **Girar**, **cambiar el tamaño** y **extraer el texto** (reconocimiento de caracteres): botones de la barra superior.
## Guardar y compartir
- `Ctrl + S` guarda, `Ctrl + Mayús + S` guarda con otro nombre o formato.
- `Ctrl + C` copia la imagen anotada al portapapeles.
- **Efectos al exportar** (borde, sombra paralela): se añaden a la imagen guardada o copiada.
- `Ctrl + rueda` hace zoom, `Ctrl + 0` ajusta a la ventana; el botón central del ratón desplaza la vista.

# El historial de capturas
Inicio muestra las capturas de la carpeta, las más recientes primero.
- **Filtros** Todo / Imágenes / Vídeos y **búsqueda** por nombre.
- **Abrir**: doble clic (editor para una imagen, reproductor para un vídeo).
- **Acciones rápidas** al pasar sobre una miniatura: editar, copiar, anclar, mostrar en el Explorador, eliminar. Clic derecho: menú completo.
## Seleccionar varias capturas
- **La casilla redonda** arriba a la izquierda de una miniatura la añade a la selección con un simple clic. Después, un clic en otra miniatura la añade o la quita, sin mantener ninguna tecla.
- O bien: `Ctrl` + clic para añadir o quitar, `Mayús` + clic para un rango, `Ctrl + A` para seleccionar todo.
- La **barra de selección** muestra el número de capturas y permite seleccionar todo, anclar o **eliminar toda la selección** con un clic (o `Supr`). `Esc` anula la selección.
- Las capturas eliminadas van a la Papelera, tras confirmación.
## Añadir imágenes
- `Ctrl + V` pega la imagen del portapapeles en la carpeta de capturas.
- Arrastrar archivos de imagen desde el Explorador a la ventana los añade.
- El botón «Abrir una imagen» abre cualquier imagen en el editor.

# Grabar un vídeo
- Iniciar: `Ctrl + Mayús + Impr Pant`, la tarjeta **Vídeo** de Inicio o el menú de notificación, y elegir la zona como para una captura.
- **Sonido**: la tecla `M` (o un clic en la etiqueta) elige con o sin micrófono antes de empezar. Durante la grabación, el botón de micrófono de la barra flotante silencia o recupera el sonido.
- **Detener**: botón de la barra flotante o el mismo atajo. **Cancelar**: `Esc`.
- Formatos MP4, WebM o GIF, fotogramas por segundo, cursor y micrófono: en Ajustes. La grabación usa FFmpeg (propuesto por el instalador).

# Conservación de capturas
En Ajustes, ScreenCapture puede eliminar automáticamente las capturas antiguas:
- una **duración** para las imágenes y otra para los vídeos (7, 30 o 90 días, 1 año, duración personalizada o nunca);
- un **tamaño máximo** de la carpeta: por encima, se eliminan primero las más antiguas;
- a la **Papelera** (recuperable) o **definitivamente**.
Las capturas **ancladas** (chincheta en la miniatura) y la abierta en el editor nunca se eliminan; los archivos que no proceden de ScreenCapture nunca se tocan. «Limpiar ahora» muestra lo que se va a eliminar antes de hacerlo.

# Ajustes
![parametres]
- **Captura**: carpeta, formato PNG o JPEG, guardado y copia automáticos, qué ocurre tras la captura, cursor, retardo de la captura diferida, lupa.
- **Atajos de teclado**: escribir un atajo (por ejemplo `ctrl+shift+KeyS`) y pulsar «Aplicar»; dejar vacío para desactivarlo.
- **Colores del editor**: modificar, añadir o quitar los colores propuestos.
- **Actualizaciones**: búsqueda automática y botón «Buscar ahora».
- **Apariencia e inicio**: idioma (francés, inglés, español, alemán o el del sistema), tema claro u oscuro, color de énfasis, cierre en el área de notificación, inicio minimizado.
- **Iniciar al arrancar Windows** (o al abrir sesión en Mac): se activa o desactiva en cualquier momento, aunque se haya elegido en la instalación. El interruptor muestra el estado real.

# Actualizaciones y versión portátil
## Actualizaciones
Cuando se publica una nueva versión, aparece una barra en la parte superior de la ventana. **Actualizar** la descarga, comprueba su huella, la instala y vuelve a abrir ScreenCapture. **Más tarde** oculta la barra hasta el próximo inicio. En Mac, la imagen de disco se descarga y se abre: basta con arrastrar la aplicación a Aplicaciones.
## Versión portátil
La versión portátil (archivo `.zip`) se usa sin instalación, por ejemplo desde una memoria USB: descomprimir la carpeta y ejecutar `ScreenCapture.exe`. Los ajustes y el registro se guardan en la carpeta `data` junto al programa. Las actualizaciones sustituyen el programa en el mismo lugar.

# Para ir más lejos
## Línea de comandos
Una instancia ya abierta se controla con `ScreenCapture.exe --cmd` seguido de: `region`, `full`, `window`, `last`, `delayed`, `video`, `show`, `hide`, `settings`, `cancel` o `quit`. Práctico para un atajo personalizado o un script.
## Si hay un problema
- Si la aplicación deja de responder, volver a abrirla desde el menú Inicio sustituye la instancia bloqueada.
- Se puede activar un registro de diagnóstico con un triple clic en la versión, en «Acerca de».
- ScreenCapture es software libre (licencia GPL-3.0): el código fuente y las versiones están en GitHub.
## En Mac: permiso de grabación de pantalla
ScreenCapture necesita el permiso **Grabación de pantalla** para las capturas y los vídeos (FFmpeg, iniciado por la aplicación, también depende de él). Sin él, las capturas salen **negras** o solo muestran el fondo de pantalla.
- Cuando falta el permiso, aparece una franja roja en la parte superior de la ventana con **Abrir Ajustes** y **Reiniciar**.
- En Ajustes del Sistema > Privacidad y seguridad > Grabación de pantalla, si ScreenCapture ya está en la lista pero las capturas siguen negras, eliminar la entrada con el botón `−`, volver a añadirla con `+` (o activarla) y reiniciar la aplicación.
- El sonido del sistema necesita además el permiso **Solo grabación de audio del sistema**, en la misma página.
> Desde macOS Sequoia, el sistema vuelve a pedir el permiso aproximadamente una vez al mes a cada aplicación que graba la pantalla. Es normal: basta con aceptar.
