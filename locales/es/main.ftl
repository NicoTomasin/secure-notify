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
