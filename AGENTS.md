# process

- when writing commit messages, follow git best practices.

when starting a new project, do things right:
  - use automatic linting and formatting, on save (set in VS Code settings) and on commit if possible.
  - set up unit tests, and integration/end-to-end tests for each target platform.
  - use continuous integration, particularly repositories that can be public (and use GitHub Actions for free)
  - consider if the app will need features down the line that have architectural implications (for example, like localization, internationalization, accessibility, cross-platform support, etc.), and if so, design for them now.
  - as soon as you can, set up a solid release process, error reporting, monitoring, analytics, and anything else necessary for a production-ready product.

# subagents

send this AGENTS.md document and any rules files to any subagents who may do coding work.

whenever you come across a relatively straightforward or discrete subtask of implementation or investigation, consider using a subagent. when I ask for you or another agent to act as **"coordinator"**, that means I want you to use subagents in this way, you should handle mostly just systematic/design questions yourself.

# writing style

- favor a refined, economical writing style--but above all, **please talk like an actual person**.
- when speaking to me, or writing markdown, take full advantage of the markdown format, using bold text to highlight especially new information and bullet points in lieu of a paragraph when listing a series of group members, recounting a succession of events, or enumerating a list of properties. be careful to use headers that summarize rather than foreshadow, so that the reader may used them to quickly review text after reading.
- documentation and comments should be clear and easy to understand. use full sentences and steer clear of metaphor and figurative speech.
- follow the guidelines for comments outlined in [comments rules](#code-comments).

# coding guidelines

- classic wisdom
  - keep things small and manageable.
    - default to these baseline rules (adapted from Sandi Metz):
      - modules no longer than 100 lines of code
      - functions/methods no longer than five lines of code
      - no more than four parameters per function (and no working around this limit by using one big object/hash).
    - discard these rules when the indirection from extracting functions/variables actually hurts readability, e.g.:
      - within complex mathematical or algorithmic logic
      - within large but simple declarative branching code, e.g. declarative UIs (React) or declarative data transformation (Redux reducers)
    - also discard these rules when the _verified_ performance cost of function calls/object instantiation is too high
  - keep things readable.
    - descriptive naming
      - for functions, prefer verb phrases telling _what the function does_. For functions returning boolean values, write them in question form, e.g. `isDueToday`, `hasNeighbor`, etc.
      - whenever a noun enters a name, be specific and consistent about singular vs. plural usage, e.g. `dogCollars` if there is only one dog in question vs . `dogsCollars` if there may be more than one.
      - for files, match the name of the primary export, be it a function, constant, type, etc., or else choose a name that tells clearly _what the contents do_ or what _specific purpose_ the contents serve. If the best name for a file seems to be something like _helpers.ts_, that's likely a sign the contents should distributed to different files, i.e. nearer to where they are used. If you are tempted to name the file _type.ts_, the _outermost type_ in nested structures can be a good candidate for determining the filename.
      - avoid single-letter names, except when they help readability (e.g. inside a lambda, where the body is only a few characters long minus the parameter name)
    - comments where descriptive naming does not suffice, e.g.:
      - when the performance hit from variable assignment is undesirable
      - doc comments especially for functions and values at "library-like" interfaces
      - to clarify complex mathematical or algorithmic logic within a function
    - follow a logical order
      - within a file, put the important stuff first.
        - you shouldn't have to scroll down to see the "main" function or class in a file.
        - small bits of "configuration" (e.g. constant declarations) may go at the top of the file before the "main" function.
      - put parameters, declarations, import statements etc. from "generic" to "specific".
    - keep files organized.
      - use a logical folder structure, developed gradually as files are added and sensible groupings become clear.
      - take advantage of *prefixing* to keep related files together in an alphabetical listing, and to avoid overly complicated nested folder structures. the prefixes you choose should produce names that still read as proper english.
      - only add a prefix or a new folder when a folder's contents grow large enough to warrant it.
  - don't prematurely optimize
    - the most performant code can be hard to read/maintain, so make sure to verify bottlenecks first.
    - don't abstract too early: DRY up code only when the repetition is large or more than twice.
    - in general, get it working first. only after tests are passing should you start splitting things up into smaller functions/modules.
  - keep things organized
    - separation of concerns
    - DRY
    - composition over inheritance
- prefer pure functions to isolate side effects
  - use immutable data whenever possible (except when performance concerns disallow it). the codebase should be mostly *pure functions*, which are easier to test, debug, and scale, with code triggering *side effects* isolated to the edges of the system.
  - by saying that this purity serves _to isolate side effects_, we are giving ourselves permission to use mutability sparingly when it helps readability. don't fight the language--things like `for` loops in JavaScript/TypeScript are fine. even classes can be OK
- test as you code
  - include tests for all important functionality.
  - the priority should be a few full end-to-end/integration/feature tests for the most important paths, but most tests should be small unit tests.
  - tests are code, so follow the same guidelines as you would for other parts of your codebase.
  - specifics:
    - use one expectation per test case.
    - don't use shared state between tests unless this addresses a proven performance bottleneck. prefer repeated calls to setup functions.
    - keep tests near to the relevant code. Where it's practical (e.g. for most unit tests), name the test file the same as the file with the tested code, but with `.test.ts` instead of `.ts`.

# code comments

Firstly, freely delete any redundant comments you come across. when well thought-out names, type signatures, and structure would make a comment redundant, _let the code speak for itself_.

when to comment:

- comment very sparingly.
  - favor doc comments for declarations.
  - add inline comments only when needed to clarify complex logic or the reasoning behind a particular implementation choice.

writing style:

- keep comments concise, but clear. use complete sentences or at least verb phrases (for comments explaining what a function does).
- avoid metaphor, aphorism, and any other literary flourishes. favor a plain, direct style that is easy to read and understand.
- assume the reader is not a domain expert, and make references to domain-specific concepts explicit.

content of comments:

- in doc comments, always ask, "does _the consumer of this code_ absolutely need to know this?"
- write comments that won't soon go out of date.
  - limit any mentions of implementation details of a function or method being commented on.
  - avoid references to specific code or documentation, except when the reference is _the essential basis for the commented code_.

line breaks:

- in a comment that spans several lines, break lines only at the end of a sentence or clause. never break in the middle of a phrase to fill a column width; a shorter line is preferable.
