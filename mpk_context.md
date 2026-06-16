# .mpk - Molotov Package Manager Deploys

Formato de configuración propio para el package manager de Molotov (mpm).
Parser separado del compilador principal.

## Formato

```
# Comentario con #
package.name        mi-libreria
package.version     1.2.0
package.author      jzadl
package.description "la mejor librería"

deps.sysapi         jzadl/sys-api@2.3
deps.multibuild     rusty/mltv-multibuild@5.0.1

scripts.build       mltv deploy
scripts.test        mltv run tests/
```

## Reglas

- Separador: **1 solo espacio** entre clave y valor
- Jerarquía: **dot notation** (`seccion.clave valor`)
- Strings con espacios: **comillas dobles `""`**
- Strings sin espacios: sin comillas
- Scripts: todo lo que va después del primer espacio es el valor completo (sin comillas)
- Comentarios: `#`
- Líneas vacías: ignoradas
- Versiones: se aceptan `2.3` y `2.3.0` (patch opcional)
- Formato de dependencia: `usuario/repo@version`

## Parser (pseudocódigo)

```
para cada línea:
    ignorar si vacía o empieza con #
    split por primer "." → sección
    split por primer " " → clave / valor
    si valor empieza con '"' → leer hasta '"' de cierre
    si es script → el valor es todo el resto de la línea
```

## Por qué este formato
- Descartado TOML (muy estándar)
- Descartado YAML (bugs con indentación)
- Descartada indentación significativa (incómodo)
- Descartadas llaves `{}` (desorden visual)
- Dot notation: flat, una línea = un valor, parser trivial, escala con más niveles sin tocar la lógica
