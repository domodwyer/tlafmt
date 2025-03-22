----------------------------- MODULE EWD998_optsSC -----------------------------
EXTENDS TLC, IOUtils, Naturals, Sequences, CSV

Features ==
    \* Features is redundantly defined in EWD998_opts.tla.  Could be extracted
    \* into common EWD998_opts_frob.ta, though.
    {"pt1","pt2","pt3","pt4","pt5"}
        
Nodes ==
    {7,29,43}
    \* {7, 19, 29, 37, 43}

-----------------------------------------------------------------------------

\* Filename for the CSV file that appears also in the R script and is passed
\* to the nested TLC instances that are forked below.
CSVFile ==
    "EWD998_opts_" \o ToString(JavaTime) \o ".csv"

\* Write column headers to CSV file at startup of TLC instance that "runs"
\* this script and forks the nested instances of TLC that simulate the spec
\* and collect the statistics.
ASSUME 
    CSVWrite("Variant#Node#Length#T#T2TD#InitiateProbe#PassToken#SendMsg#RecvMsg#Deactivate",
             <<>>, CSVFile)

\* Command to fork nested TLC instances that simulate the spec and collect the
\* statistics. TLCGet("config").install gives the path to the TLC jar also
\* running this script.
Cmd == LET absolutePathOfTLC == TLCGet("config").install
       IN <<"java", "-jar",
          absolutePathOfTLC, 
          "-deadlock", "-noTE",
          "-depth", "-1",
          "-workers", "auto",
          "-generate", "num=10",
          "-config", "EWD998_opts.tla",
          "EWD998_opts.tla">>

ASSUME \A features \in SUBSET Features:
            \A n \in Nodes:
    LET ret == IOEnvExec([N |-> n, F |-> features, Out |-> CSVFile], Cmd).exitValue
    IN CASE ret =  0 -> PrintT(<<JavaTime, n, features>>)
         [] ret = 10 -> PrintT(<<n, features, "Assumption violation">>)
         [] ret = 12 -> PrintT(<<n, features, "Safety violation">>)
         [] ret = 13 -> PrintT(<<n, features, "Liveness violation">>)
         \* For all other error codes, print TLC's error message.
         [] OTHER    -> Print(<<n, features, 
                                IOEnvExec([N |-> n, F |-> features, Out |-> CSVFile], Cmd),
                                "Error">>, FALSE)

=============================================================================
---- CONFIG EWD998_optsSC ----
\* TLC always expects a configuration file, but an empty one will do in this case.
\* If this approach proves useful, TLC should be extended to launch without a config
\* file.
====

$ tlc -config EWD998_optsSC.tla EWD998_optsSC.tla