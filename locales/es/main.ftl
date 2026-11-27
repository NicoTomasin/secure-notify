# SPDX-License-Identifier: AGPL-3.0-or-later
# Spanish translation of locales/en/main.ftl.

language-name = Español

## Command descriptions shown by /help

command-start = Unirte a esta instancia con un código
command-help = Ver los comandos que podés usar
command-about = Licencia, versión y código fuente
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
