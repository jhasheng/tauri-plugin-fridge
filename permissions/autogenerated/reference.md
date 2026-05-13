## Default Permission

Default permissions for the fridge plugin — grants every command. Suitable
for apps that own the JS runtime end-to-end. If you ship a Tauri shell that
loads untrusted content, narrow this down by listing only the commands you
actually need (e.g. just `allow-list-captures`).

#### This default permission set includes the following:

- `allow-compile-script`
- `allow-start-capture`
- `allow-stop-capture`
- `allow-list-captures`

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`fridge:allow-compile-script`

</td>
<td>

Enables the compile_script command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:deny-compile-script`

</td>
<td>

Denies the compile_script command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:allow-list-captures`

</td>
<td>

Enables the list_captures command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:deny-list-captures`

</td>
<td>

Denies the list_captures command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:allow-start-capture`

</td>
<td>

Enables the start_capture command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:deny-start-capture`

</td>
<td>

Denies the start_capture command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:allow-stop-capture`

</td>
<td>

Enables the stop_capture command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fridge:deny-stop-capture`

</td>
<td>

Denies the stop_capture command without any pre-configured scope.

</td>
</tr>
</table>
