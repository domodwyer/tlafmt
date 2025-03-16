------------------------------- MODULE EWD998Chan_opts -------------------------------
EXTENDS EWD998Chan, TLC, TLCExt, IOUtils, CSV

\* The data collection below only works with TLC running in generation mode.
\* Unless TLC runs with -Dtlc2.tool.impl.Tool.probabilistic=true (or -generate),
\* the simulator generates all successor states from which it chooses one randomly. 
\* In "generate" mode, however, TLC randomly generates a (single) successor state.
ASSUME TLCGet("config").mode = "generate"
\* Do not artificially restrict the length of behaviors.
ASSUME TLCGet("config").depth = -1
\* The algorithm terminates. Thus, do not check for deadlocks.
ASSUME TLCGet("config").deadlock = FALSE
\* Require a recent versions of TLC with support for the operators appearing below.
ASSUME TLCGet("revision").timestamp >= 1628119427 

--------------------------------------------------------------------------------

\* A set of "feature flags". {} is equal to vanilla EWD998. pt 1 to 4 are
\* algorithm variants that might or might not make the algorithm more efficient.
\* We define efficient here as how quickly termination is detected and how many
\* rounds are executed, i.e., how many  InitiateProbe  actions occur on average.
\* The way we are going to measure efficiency is by measuring the number of 
\* steps between termination of  Environment  and the termination detection
\* by  System  .  Additionally, we will count the number of all actions.
\*
\* pt1, pt2: An active node may pass the token if the node or token are tainted.
\* pt3: Return the token to the initiator and, thus, short-circuits the token
\* round, iff the node is black.
\* pt4: Return the token to the initiator iff the token is black.
\*
\* (pt3 and pt4 can be described as "aborting" an inconclusive token round by 
\* directly returning the token to the initiator).
\*
\* The variants pt3 & pt4 come at the cost of all nodes knowing the identify of
\* the initiator.  However, this would be trivially addressed by stamping the
\* initiator's id onto the token.
\*
\* As we do not model a particular workload by constraining the behaviors
\* satisfying  Environment, the TLC generator has to run long enough to create a
\* sufficient amount of traces.
FeatureFlags == 
    {"pt1","pt2","pt3","pt4"}

CONSTANT F
ASSUME F \subseteq FeatureFlags

--------------------------------------------------------------------------------

InitSim ==
    \* Constraint the set of initial states defined in EWD998!Init to
    \* those that correspond to what an implementation is likely to start with.
    \* In other words, when collecting statistics, we don't want to start in a
    \* state where the system has e.g. already terminated.
    /\ active = [n \in Node |-> TRUE]
    /\ color = [n \in Node |-> "white"]

PassTokenOpts(n) ==
  /\ n # 0
  (* Rule 2 *)
  /\ \/ ~ active[n] \* If machine i is active, keep the token.
     \/ /\ "pt1" \in F
        /\ color[n] = "black"
     \/ /\ "pt2" \in F
        /\ \E j \in 1..Len(inbox[n]) : inbox[n][j].type = "tok" /\ inbox[n][j].color = "black"
  /\ \E j \in 1..Len(inbox[n]) : 
          /\ inbox[n][j].type = "tok"
          \* the machine nr.i+1 transmits the token to machine nr.i under q := q + c[i+1]
          /\ LET tkn == inbox[n][j]
             IN  inbox' = [inbox EXCEPT ![CASE "pt3" \in F /\ color[n] = "black" -> 0
                                            [] "pt4" \in F /\ tkn.color ="black" -> 0
                                            [] OTHER    ->  n-1] = 
                                       Append(@, [tkn EXCEPT !.q = tkn.q + counter[n],
                                                             !.color = IF color[n] = "black"
                                                                       THEN "black"
                                                                       ELSE tkn.color]),
                                    ![n] = RemoveAt(@, j) ] \* pass on the token.
  (* Rule 7 *)
  /\ color' = [ color EXCEPT ![n] = "white" ]
  \* The state of the nodes remains unchanged by token-related actions.
  /\ UNCHANGED <<active, counter>>                    

SystemOpts(n) == \/ InitiateProbe
                 \/ PassTokenOpts(n)

SpecOpts == InitSim /\ Init /\ [][\E n \in Node: SystemOpts(n) \/ Environment]_vars

--------------------------------------------------------------------------------

\* Initialize TLC register.
ASSUME TLCSet(1, 0)

AtTermination ==
    IF EWD998!Termination # EWD998!Termination'
    THEN TLCSet(1, TLCGet("level"))
    ELSE TRUE

AtTerminationDetected ==
    \* This is just an ordinary state constraint (could have been an invariant 
    \* too).  The disadvantage of a constraint (or inv) is that the antecedent
    \* is evaluated for *every* generated state, instead of just after the last
    \* state when we actually want the consequent to be evalauted.
    \* A constraint's advantage is that it works with old versions of TLC.
    EWD998!terminationDetected =>
    /\ LET o == TLCGet("stats").behavior.actions
       IN \* Append record to CSV file on disk.
          /\ CSVWrite("%1$s#%2$s#%3$s#%4$s#%5$s#%6$s#%7$s#%8$s#%9$s#%10$s",
               << F, N, TLCGet("level"), TLCGet(1), TLCGet("level") - TLCGet(1),
                 o["InitiateProbe"],o["PassTokenOpts"], \* Note "Opts" suffix!
                 o["SendMsg"],o["RecvMsg"],o["Deactivate"]
                 >>,
               IOEnv.Out)
          \* Reset the counter for the next behavior.
          /\ TLCSet(1, 0)

--------------------------------------------------------------------------------

Features ==
    CHOOSE s \in SUBSET FeatureFlags : ToString(s) = IOEnv.F

Nodes ==
    CHOOSE n \in 1..256 : ToString(n) = IOEnv.N

================================================================================

------------------------------ CONFIG EWD998_opts ------------------------------
CONSTANTS
    N <- Nodes
    F <- Features

SPECIFICATION 
    SpecOpts

ACTION_CONSTRAINT
    AtTermination

CONSTRAINT
    AtTerminationDetected
================================================================================