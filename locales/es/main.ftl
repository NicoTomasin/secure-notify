# SPDX-License-Identifier: AGPL-3.0-or-later
# Spanish translation of locales/en/main.ftl.

language-name = Español

## Command descriptions shown by /help

command-start = Unirte a esta instancia con un código
command-help = Ver los comandos que podés usar
command-about = Licencia, versión y código fuente
command-upload = Cómo mandar un manifiesto o lockfile
command-inventory = Ver, agregar o quitar entradas del inventario
command-watch = Seguir una tecnología
command-unwatch = Dejar de seguir algo
command-watching = Ver lo que seguís
command-rules = Ver tus reglas y fijar una severidad mínima
command-language = Ver o cambiar el idioma del bot
command-invite = Crear un código de invitación para una persona (admins)
command-subscribers = Ver los suscriptores (admins)
command-remove = Eliminar un suscriptor y sus datos (admins)

## /start

start-admin = Ahora sos admin de esta instancia.
start-member = ¡Bienvenido! Te uniste a esta instancia.
start-already-registered = Ya estás registrado. Mandá /help para ver los comandos.
onboarding =
    Qué guarda este bot sobre vos: tu ID de chat en esta plataforma, tu idioma, lo que seguís y el inventario que subís o agregás, reducido a nombres de componentes y versiones. Los archivos subidos nunca se guardan y el inventario no se agrupa por proyecto.
    Quien opera esta instancia puede leer esos datos. Mandá /help para empezar.

## /help

help-header = Comandos que podés usar:
help-line = /{ $command } — { $description }

## /about

about-version = Versión: { $version }
about-license = Licencia: { $license }
about-source = Código fuente: { $url }

## /upload and uploads

upload-prompt = Mandá uno de estos archivos en este chat: { $files }. Se lee una vez y nunca se guarda; solo se conservan nombres de componentes y versiones.
upload-done = Inventario actualizado con { $file }: { $added } nuevos, { $refreshed } ya conocidos.
upload-technologies = Tecnologías detectadas, ahora seguidas: { $technologies }.
upload-unsupported = No puedo leer { $file }. Archivos soportados: { $files }.
upload-too-large = El archivo pesa más de { $limit } MB.
upload-too-many = El archivo lista más de { $limit } componentes.
upload-timeout = Leer el archivo tardó demasiado. Probá con uno más chico.
upload-malformed = No se pudo leer { $file }; no parece un archivo válido de su tipo.
upload-bun-binary = El lockfile binario de Bun (bun.lockb) no está soportado. Bun 1.2 o posterior escribe el lockfile de texto bun.lock; con versiones anteriores corré “bun install --save-text-lockfile”. Después mandá bun.lock.

## /inventory

inventory-empty = Tu inventario está vacío. Mandá un lockfile o usá /inventory add <purl> <versión>.
inventory-header = { $count ->
    [one] Una entrada:
   *[other] { $count } entradas:
}
inventory-no-version = (sin versión)
inventory-more = …y { $count } más.
inventory-usage = Usá /inventory add <purl> [versión], /inventory remove <purl>[@versión] o /inventory clear.
inventory-added = Agregado { $component } { $version }.
inventory-invalid = “{ $component }” no es un package URL como pkg:npm/lodash.
inventory-removed = { $count ->
    [one] Se quitó una entrada.
   *[other] Se quitaron { $count } entradas.
}
inventory-cleared = Se vació tu inventario ({ $count } entradas quitadas).

## /watch, /unwatch, /watching

watch-usage = Uso: /watch <nombre>, por ejemplo /watch Node.js. Podés seguir: { $available }.
watch-added = Ahora seguís { $name }.
watch-already = Ya seguís { $name }.
watch-unknown = “{ $query }” no está en el catálogo. Podés seguir: { $available }.
unwatch-usage = Uso: /unwatch <nombre>, como aparece en /watching.
unwatch-removed = Ya no seguís { $name }.
unwatch-not-watched = No seguís { $name }.
watching-none = Todavía no seguís nada. Usá /watch <nombre>.
watching-header = Seguís:
watching-line = { $name } ({ $id }){ $origin ->
    [derived] {" "}· por tus archivos subidos
   *[manual] {""}
}

## /rules

rules-header = { $default ->
    [yes] Tus reglas (las predeterminadas), en orden; decide la primera que coincide:
   *[no] Tus reglas, en orden; decide la primera que coincide:
}
rule-notify = { $position }. Notificar cuando { $conditions }.
rule-notify-min = { $position }. Notificar cuando { $conditions }, si la severidad es { $level } o mayor.
rule-ignore = { $position }. Ignorar cuando { $conditions }.
rules-min-severity = Severidad mínima: { $level }. Las historias explotadas se notifican siempre.
rules-min-severity-none = No hay severidad mínima.
rules-usage = Usá /rules severity <critical|high|medium|low|off> o /rules reset.
rules-severity-set = Severidad mínima fijada en { $level }. Las historias explotadas se siguen notificando.
rules-severity-off = Se quitó la severidad mínima.
rules-severity-invalid = “{ $level }” no es un nivel. Usá critical, high, medium, low u off.
rules-reset = Tus reglas volvieron a las predeterminadas.

## /language

language-current = Idioma actual: { $language }. Disponibles: { $available }.
language-usage = Cambialo con /language <código>, por ejemplo /language en.
language-changed = Idioma cambiado a { $language }.
language-unknown = “{ $code }” no es un idioma disponible. Disponibles: { $available }.

## /invite

invite-created = Código de invitación: { $code }
invite-instructions = La persona manda /start { $code } a este bot en un chat privado. El código sirve una sola vez y vence el { $expires }.

## /subscribers

subscribers-header = { $count ->
    [one] Un suscriptor:
   *[other] { $count } suscriptores:
}
subscribers-line = #{ $id } · { $role } · { $language } · desde { $since }{ $active ->
    [no] {" "}· inactivo
   *[yes] {""}
}
role-admin = admin
role-member = miembro

## /remove

remove-usage = Uso: /remove <número de suscriptor>, como aparece en /subscribers.
remove-done = Se eliminó el suscriptor #{ $id } y sus datos.
remove-not-found = No existe el suscriptor #{ $id }.
remove-self = No podés eliminarte a vos mismo.

## Errors

error-permission-denied = Solo los admins pueden usar /{ $command }.
error-unknown-command = Comando desconocido. Mandá /help para ver los comandos.
error-internal = Algo salió mal. Probá de nuevo más tarde.

## Notifications

severity-critical = CRÍTICA
severity-high = ALTA
severity-medium = MEDIA
severity-low = BAJA
severity-unknown = SEVERIDAD DESCONOCIDA
kind-vulnerability = Vulnerabilidad
kind-exploit = Exploit
kind-malicious-package = Paquete malicioso
kind-release = Versión de seguridad
kind-bug = Bug crítico
kind-breach = Filtración
kind-attack = Ataque
kind-outage = Caída
kind-news = Noticia
notice-header = [{ $severity }] { $kind }: { $title }
notice-update-header = Actualización · [{ $severity }] { $kind }: { $title }
notice-exploited = Se sabe que está siendo explotada.
notice-exploit-available = Hay un exploit público.
change-severity = La severidad subió a { $severity }.
change-exploited = Ahora se sabe que está siendo explotada.
change-exploit-available = Ahora hay un exploit público.
change-fix-available = Ahora hay una corrección.
reason-version-fixed = En tu inventario: { $component } { $version } (corregido en { $fixed }).
reason-version = En tu inventario: { $component } { $version }.
reason-component = En tu inventario: { $component } (versión desconocida).
reason-watch = Seguís { $name }.
field-cvss = CVSS
field-epss = EPSS
notice-sources = { $count ->
    [one] Lo reporta una fuente.
   *[other] Lo reportan { $count } fuentes.
}
notice-link = Detalles
action-mute-story = Silenciar esta historia
action-mute-component = Silenciar { $component }
action-no-longer-used = Ya no uso { $component } { $version }
action-mute-entity = Silenciar { $name }

## /mutes

command-mutes = Ver, agregar y quitar silencios
mutes-none = No tenés silencios.
mutes-header = Tus silencios:
mutes-line = #{ $id } · { $scope } { $value }
mutes-line-until = #{ $id } · { $scope } { $value } hasta { $until }
mutes-usage = Usá /mutes add story <id>, entity <id>, component <purl> o version <purl@versión>, con una duración opcional como 7d; /mutes remove <número>.
mutes-added = Silenciado { $scope } { $value }.
mutes-added-until = Silenciado { $scope } { $value } hasta { $until }.
mutes-invalid = Ese silencio no es válido.
mutes-removed = Se quitó el silencio #{ $id }.
mutes-not-found = No existe el silencio #{ $id }.
action-details = Detalles
details-gone = Esa historia ya no está disponible.
details-identifiers = Identificadores: { $ids }
details-sources = Fuentes: { $sources }
details-affected = Afectado: { $component } (corregido en { $fixed })
details-affected-unfixed = Afectado: { $component } (todavía sin corrección)
details-related = Relacionada: { $title }

## Initial scan

upload-scan-deferred = Los datos de vulnerabilidades todavía se están sincronizando por primera vez. El reporte del escaneo inicial llegará cuando termine.
scan-none = Escaneo inicial de { $entries ->
    [one] una entrada
   *[other] { $entries } entradas
}: ningún problema conocido que no se te haya informado antes.
scan-header = Escaneo inicial de { $entries ->
    [one] una entrada
   *[other] { $entries } entradas
}: { $count ->
    [one] un problema conocido que no se te había informado.
   *[other] { $count } problemas conocidos que no se te habían informado.
}
scan-story = [{ $severity }] { $title }: { $entries }
scan-story-exploited = [{ $severity }, explotada] { $title }: { $entries }
scan-entry-fixed = { $component } { $version }, corregido en { $fixed }
scan-entry = { $component } { $version }, todavía sin corrección
scan-entry-unknown = { $component } (versión desconocida)
