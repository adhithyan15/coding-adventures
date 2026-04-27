package com.codingadventures.graph;

import java.util.List;

public interface TraversalGraph {
    boolean hasNode(String node);
    List<String> nodes();
    List<String> neighbors(String node);
    int size();
}
