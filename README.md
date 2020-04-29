# hamster

offline gitlab runner

**ALPHA**

Tired of having to 'register' runners?

Just want a runner that doesn't run in a container, but tries to just, you know, run the stuff?

# But I can't change my .gitlab-ci.yml

No worries, if there's a .gitlab-local.yml it will use that in preference and from there you can include your .gitlab-ci.yml. That way you can override global variables that aren't quite right.

# What does hamster honor?

  * `hamster target_name` will run that specific target.
  * variables defined at job and global level will be honored.
  * variable substitution works in the same mannor as go expand.
  * .extends is now supported.
  * yaml merge << and anchors work.

E.g. with this for your `.gitlab-ci.yml`:
```
goodbye:
  stage: primary_stage
  variables:
    GOODBYE: "tara"
  script:
    - echo $GOODBYE a bit
```
then `hamster goodbye` would output `tara a bit`.

# What doesn't it do?

Does't honor:

   * services
   * image
   * when
  * `hamster stage_name` will not run all targets in a stage.[todo]

It won't checkout your code or do anything with git.

It won't start itself in a container (use the official gitlab runner for that)

# Changelog

   * vNext
    - unix shell defaults to bash rather than sh.
    - check for presence of a .gitlab-local.yml
   * v0.0.4 Bugfix for same dir includes.
   * v0.0.3 Intial release