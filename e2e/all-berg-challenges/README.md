
The idea behind this test is to test all challenges in the berg namespace.

This should
- check which challenges can be instanced (`len(containers) > 0`)
- create a challenge instance for each challenge (sequentially?)
- ensure the challenge instance comes up running
- run our baseline tests
- do some connectivity tests, eg. curl -k on http paths

Since we cannot do simple loops with a chainsaw test, it's likely easier to loop over the challenges with bash
