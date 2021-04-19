import {
  FETCH_LIBRARIES_START,
  FETCH_LIBRARIES_OK,
  FETCH_LIBRARIES_ERR,
  FETCH_LIBRARY_INFO,
  FETCH_LIBRARY_MEDIA,
  NEW_LIBRARY_START,
  NEW_LIBRARY_OK,
  NEW_LIBRARY_ERR,
  DEL_LIBRARY_START,
  DEL_LIBRARY_OK,
  DEL_LIBRARY_ERR,
  RM_LIBRARY,
  ADD_LIBRARY,
  FETCH_LIBRARY_UNMATCHED_START,
  FETCH_LIBRARY_UNMATCHED_ERR,
  FETCH_LIBRARY_UNMATCHED_OK
} from "./types.js";

export const fetchLibraries = (token) => async (dispatch) => {
  dispatch({ type: FETCH_LIBRARIES_START });

  try {
    const config = {
      headers: {
        "authorization": token
      }
    };

    const res = await fetch(`//${window.host}:8000/api/v1/library`, config);

    if (res.status !== 200) {
      return dispatch({
        type: FETCH_LIBRARIES_ERR,
        payload: res.statusText
      });
    }

    const payload = await res.json();

    dispatch({
      type: FETCH_LIBRARIES_OK,
      payload
    });
  } catch(err) {
    dispatch({
      type: FETCH_LIBRARIES_ERR,
      payload: err
    });
  }
};

export const fetchLibraryUnmatched = (token, id) => async (dispatch) => {
  dispatch({ type: FETCH_LIBRARY_UNMATCHED_START });

  try {
    const config = {
      headers: {
        "authorization": token
      }
    };

    const res = await fetch(`//${window.host}:8000/api/v1/library/${id}/unmatched`, config);

    if (res.status !== 200) {
      return dispatch({
        type: FETCH_LIBRARY_UNMATCHED_ERR,
        payload: res.statusText
      });
    }

    const payload = await res.json();

    dispatch({
      type: FETCH_LIBRARY_UNMATCHED_OK,
      payload
    });
  } catch(err) {
    dispatch({
      type: FETCH_LIBRARY_UNMATCHED_ERR,
      payload: err
    });
  }
};

export const newLibrary = (token, data) => async (dispatch) => {
  dispatch({ type: NEW_LIBRARY_START });

  const options = {
    method: "POST",
    headers: {
      "Authorization": token,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(data)
  };

  try {
    const res = await fetch(`//${window.host}:8000/api/v1/library`, options);

    if (res.status !== 201) {
      return dispatch({
        type: NEW_LIBRARY_ERR,
        payload: res.statusText
      });
    }

    dispatch({ type: NEW_LIBRARY_OK });
  } catch(err) {
    dispatch({
      type: NEW_LIBRARY_ERR,
      payload: err
    });
  }
};

export const delLibrary = (token, id) => async (dispatch) => {
  dispatch({ type: DEL_LIBRARY_START });

  const options = {
    headers: {
      "authorization": token
    },
    method: "DELETE"
  };

  try {
    const res = await fetch(`//${window.host}:8000/api/v1/library/${id}`, options);

    if (res.status !== 204) {
      return dispatch({
        type: DEL_LIBRARY_ERR,
        payload: res.statusText
      });
    }

    dispatch({ type: DEL_LIBRARY_OK });
  } catch(err) {
    dispatch({
      type: DEL_LIBRARY_ERR,
      payload: err
    });
  }
};

export const fetchLibraryInfo = () => async (dispatch) => {
  dispatch({ type: FETCH_LIBRARY_INFO });
};

export const fetchLibraryMedia = () => async (dispatch) => {
  dispatch({ type: FETCH_LIBRARY_MEDIA });
};

export const handleWsDelLibrary = (id) => async (dispatch) => {
  dispatch({
    type: RM_LIBRARY,
    id: id,
  });
}

export const handleWsNewLibrary = (token, id) => async (dispatch) => {
  const options = {
    headers: {
      "Authorization": token
    },
    methid: "GET"
  };

  try {
    const res = await fetch(`//${window.host}:8000/api/v1/library/${id}`, options);

    if (res.status !== 200) {
      return;
    }

    const info = await res.json();

    dispatch({
      type: ADD_LIBRARY,
      payload: info
    });
  } catch(err) {}
}
